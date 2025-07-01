use anyhow::Result;
use chrono::{DateTime, Utc};
use log::debug;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::{Directed, Graph};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub type ProjectGraph = Graph<FileNode, RelationshipEdge, Directed>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMatrix {
    pub metadata: ProjectMetadata,
    pub files: HashMap<PathBuf, FileNode>,
    pub relationships: Vec<Relationship>,
    pub external_dependencies: Vec<ExternalDependency>,

    // Transient data - rebuilt on load
    #[serde(skip)]
    graph: Option<ProjectGraph>,
    #[serde(skip)]
    node_indexes: HashMap<PathBuf, NodeIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub project_root: PathBuf,
    pub scan_timestamp: DateTime<Utc>,
    pub csd_version: String,
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub plugins_used: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub hash: String,
    pub size_bytes: u64,
    pub plugin: String,
    pub language: Option<String>,
    pub is_text: bool,
    pub elements: Vec<CodeElement>,
    pub imports: Vec<Import>,
    pub exports: Vec<String>,
    pub file_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeElement {
    pub element_type: ElementType,
    pub name: String,
    pub signature: Option<String>,
    pub line_start: u32,
    pub line_end: u32,
    pub summary: Option<String>, // LLM-generated summary
    pub complexity_score: Option<u32>,
    pub calls: Vec<String>,          // Functions/methods this element calls
    pub metadata: serde_json::Value, // Plugin-specific data
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ElementType {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Interface,
    Module,
    Variable,
    Constant,
    Type,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub module: String,
    pub items: Vec<String>, // Specific items imported, if any
    pub alias: Option<String>,
    pub line_number: u32,
    pub import_type: ImportType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportType {
    Standard,   // from standard library
    ThirdParty, // from external dependency
    Local,      // from project files
    Relative,   // relative import
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from_file: PathBuf,
    pub to_file: PathBuf,
    pub relationship_type: RelationshipType,
    pub details: String,
    pub line_number: Option<u32>,
    pub strength: f32, // 0.0 to 1.0 - how strong is this relationship
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    Import,
    Call,          // Function call across files
    Inheritance,   // Class inheritance
    Configuration, // Config file affects code file
    Test,          // Test file tests source file
    Documentation, // Doc file documents source file
    Build,         // Build file affects source file
}

// For the graph edges
#[derive(Debug, Clone)]
pub struct RelationshipEdge {
    pub relationship_type: RelationshipType,
    pub strength: f32,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDependency {
    pub name: String,
    pub version: Option<String>,
    pub ecosystem: String, // "cargo", "npm", "pip", etc.
    pub dependency_type: DependencyType,
    pub source_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyType {
    Runtime,
    Development,
    Build,
    Optional,
}

impl ProjectMatrix {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            metadata: ProjectMetadata {
                project_root,
                scan_timestamp: Utc::now(),
                csd_version: env!("CARGO_PKG_VERSION").to_string(),
                total_files: 0,
                total_size_bytes: 0,
                plugins_used: Vec::new(),
            },
            files: HashMap::new(),
            relationships: Vec::new(),
            external_dependencies: Vec::new(),
            graph: None,
            node_indexes: HashMap::new(),
        }
    }

    pub fn add_file(&mut self, file_node: FileNode) {
        debug!("Adding file to matrix: {}", file_node.path.display());

        // Update metadata
        self.metadata.total_files += 1;
        self.metadata.total_size_bytes += file_node.size_bytes;

        if !self.metadata.plugins_used.contains(&file_node.plugin) {
            self.metadata.plugins_used.push(file_node.plugin.clone());
        }

        // Store the file
        self.files.insert(file_node.path.clone(), file_node);

        // Invalidate graph - will be rebuilt when needed
        self.graph = None;
        self.node_indexes.clear();
    }

    pub fn add_relationship(&mut self, relationship: Relationship) {
        debug!(
            "Adding relationship: {} -> {} ({})",
            relationship.from_file.display(),
            relationship.to_file.display(),
            serde_json::to_string(&relationship.relationship_type).unwrap_or_default()
        );

        self.relationships.push(relationship);

        // Invalidate graph
        self.graph = None;
        self.node_indexes.clear();
    }

    pub fn add_external_dependency(&mut self, dependency: ExternalDependency) {
        debug!(
            "Adding external dependency: {} from {}",
            dependency.name,
            dependency.source_file.display()
        );
        self.external_dependencies.push(dependency);
    }

    /// Save the matrix to a JSON file
    pub async fn save(&self, path: &Path) -> Result<()> {
        debug!("Saving project matrix to: {}", path.display());

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string_pretty(self)?;
        tokio::fs::write(path, json).await?;

        debug!("Matrix saved successfully");
        Ok(())
    }

    /// Load the matrix from a JSON file
    pub async fn load(path: &Path) -> Result<Self> {
        debug!("Loading project matrix from: {}", path.display());

        let json = tokio::fs::read_to_string(path).await?;
        let mut matrix: ProjectMatrix = serde_json::from_str(&json)?;

        // Rebuild the graph
        matrix.rebuild_graph();

        debug!(
            "Matrix loaded successfully with {} files",
            matrix.files.len()
        );
        Ok(matrix)
    }

    /// Rebuild the in-memory graph from the JSON data
    fn rebuild_graph(&mut self) {
        debug!("Rebuilding graph from matrix data");

        let mut graph = Graph::new();
        let mut node_indexes = HashMap::new();

        // Add all files as nodes
        for (path, file_node) in &self.files {
            let node_index = graph.add_node(file_node.clone());
            node_indexes.insert(path.clone(), node_index);
        }

        // Add relationships as edges
        for relationship in &self.relationships {
            if let (Some(&from_idx), Some(&to_idx)) = (
                node_indexes.get(&relationship.from_file),
                node_indexes.get(&relationship.to_file),
            ) {
                let edge = RelationshipEdge {
                    relationship_type: relationship.relationship_type.clone(),
                    strength: relationship.strength,
                    details: relationship.details.clone(),
                };
                graph.add_edge(from_idx, to_idx, edge);
            }
        }

        self.graph = Some(graph);
        self.node_indexes = node_indexes;

        debug!(
            "Graph rebuilt with {} nodes and {} edges",
            self.files.len(),
            self.relationships.len()
        );
    }

    /// Ensure the graph is built
    fn ensure_graph(&mut self) {
        if self.graph.is_none() {
            self.rebuild_graph();
        }
    }

    /// Find all files that depend on the given file
    pub fn find_dependents(&mut self, file_path: &Path) -> Vec<&FileNode> {
        self.ensure_graph();

        let graph = self.graph.as_ref().unwrap();
        let mut dependents = Vec::new();

        if let Some(&node_idx) = self.node_indexes.get(file_path) {
            // Find all nodes that have edges pointing TO this node
            for edge_ref in graph.edges_directed(node_idx, petgraph::Direction::Incoming) {
                let dependent_idx = edge_ref.source();
                if let Some(file_node) = graph.node_weight(dependent_idx) {
                    dependents.push(file_node);
                }
            }
        }

        dependents
    }

    /// Find all files that this file depends on
    pub fn find_dependencies(&mut self, file_path: &Path) -> Vec<&FileNode> {
        self.ensure_graph();

        let graph = self.graph.as_ref().unwrap();
        let mut dependencies = Vec::new();

        if let Some(&node_idx) = self.node_indexes.get(file_path) {
            // Find all nodes that this node has edges pointing TO
            for edge_ref in graph.edges_directed(node_idx, petgraph::Direction::Outgoing) {
                let dependency_idx = edge_ref.target();
                if let Some(file_node) = graph.node_weight(dependency_idx) {
                    dependencies.push(file_node);
                }
            }
        }

        dependencies
    }

    /// Get files by language/plugin
    pub fn get_files_by_plugin(&self, plugin_name: &str) -> Vec<&FileNode> {
        self.files
            .values()
            .filter(|file| file.plugin == plugin_name)
            .collect()
    }

    /// Calculate some basic metrics
    pub fn calculate_metrics(&mut self) -> ProjectMetrics {
        self.ensure_graph();

        let graph = self.graph.as_ref().unwrap();

        // Find files with highest in-degree (most depended upon)
        let mut coupling_scores: Vec<(PathBuf, usize)> = self
            .node_indexes
            .iter()
            .map(|(path, &idx)| {
                let in_degree = graph
                    .edges_directed(idx, petgraph::Direction::Incoming)
                    .count();
                (path.clone(), in_degree)
            })
            .collect();
        coupling_scores.sort_by_key(|(_, score)| *score);
        coupling_scores.reverse();

        ProjectMetrics {
            total_files: self.files.len(),
            total_relationships: self.relationships.len(),
            highly_coupled_files: coupling_scores.into_iter().take(10).collect(),
            languages: self.metadata.plugins_used.clone(),
        }
    }

    /// Print a summary of the matrix
    pub fn print_summary(&mut self) {
        println!("\n=== Project Matrix Summary ===");
        println!("Project: {}", self.metadata.project_root.display());
        println!(
            "Scanned: {}",
            self.metadata.scan_timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        );
        println!("Files: {}", self.metadata.total_files);
        println!(
            "Total size: {:.2} MB",
            self.metadata.total_size_bytes as f64 / (1024.0 * 1024.0)
        );
        println!("Relationships: {}", self.relationships.len());
        println!(
            "External dependencies: {}",
            self.external_dependencies.len()
        );
        println!("Languages: {}", self.metadata.plugins_used.join(", "));

        // Show files scanned by language/plugin
        println!("\n📁 Files scanned:");
        let mut by_plugin: std::collections::HashMap<String, Vec<&PathBuf>> =
            std::collections::HashMap::new();

        for (path, file_node) in &self.files {
            by_plugin
                .entry(file_node.plugin.clone())
                .or_default()
                .push(path);
        }

        let mut plugins: Vec<_> = by_plugin.keys().collect();
        plugins.sort();

        for plugin in plugins {
            let files_for_plugin = &by_plugin[plugin];
            println!(
                "  {} ({} files)",
                plugin.to_uppercase(),
                files_for_plugin.len()
            );

            // Show first few files for each plugin
            let mut sorted_files = files_for_plugin.clone();
            sorted_files.sort();
            for (i, file_path) in sorted_files.iter().enumerate() {
                if i < 5 {
                    // Show first 5 files
                    println!("    {}", file_path.display());
                } else if i == 5 {
                    println!("    ... and {} more", sorted_files.len() - 5);
                    break;
                }
            }
        }

        println!();
    }
}

#[derive(Debug)]
pub struct ProjectMetrics {
    pub total_files: usize,
    pub total_relationships: usize,
    pub highly_coupled_files: Vec<(PathBuf, usize)>,
    pub languages: Vec<String>,
}

// Unit tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Helper function to create a test FileNode
    fn create_test_file_node(path: &str, plugin: &str) -> FileNode {
        FileNode {
            path: PathBuf::from(path),
            relative_path: PathBuf::from(path),
            hash: "test_hash_123".to_string(),
            size_bytes: 1024,
            plugin: plugin.to_string(),
            language: Some(plugin.to_string()),
            is_text: true,
            elements: vec![],
            imports: vec![],
            exports: vec![],
            file_summary: Some("Test file summary".to_string()),
        }
    }

    // Helper function to create a test Relationship
    fn create_test_relationship(from: &str, to: &str) -> Relationship {
        Relationship {
            from_file: PathBuf::from(from),
            to_file: PathBuf::from(to),
            relationship_type: RelationshipType::Import,
            details: "test import".to_string(),
            line_number: Some(10),
            strength: 0.8,
        }
    }

    #[test]
    fn test_project_matrix_creation() {
        let project_root = PathBuf::from("/test/project");
        let matrix = ProjectMatrix::new(project_root.clone());

        assert_eq!(matrix.metadata.project_root, project_root);
        assert_eq!(matrix.metadata.total_files, 0);
        assert_eq!(matrix.metadata.total_size_bytes, 0);
        assert!(matrix.files.is_empty());
        assert!(matrix.relationships.is_empty());
        assert!(matrix.external_dependencies.is_empty());
        assert_eq!(matrix.metadata.csd_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_add_file() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));
        let file_node = create_test_file_node("src/main.rs", "rust");

        matrix.add_file(file_node.clone());

        assert_eq!(matrix.metadata.total_files, 1);
        assert_eq!(matrix.metadata.total_size_bytes, 1024);
        assert!(matrix.metadata.plugins_used.contains(&"rust".to_string()));
        assert!(matrix.files.contains_key(&PathBuf::from("src/main.rs")));
    }

    #[test]
    fn test_add_multiple_files_different_plugins() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));
        let rust_file = create_test_file_node("src/main.rs", "rust");
        let python_file = create_test_file_node("script.py", "python");

        matrix.add_file(rust_file);
        matrix.add_file(python_file);

        assert_eq!(matrix.metadata.total_files, 2);
        assert_eq!(matrix.metadata.total_size_bytes, 2048);
        assert_eq!(matrix.metadata.plugins_used.len(), 2);
        assert!(matrix.metadata.plugins_used.contains(&"rust".to_string()));
        assert!(matrix.metadata.plugins_used.contains(&"python".to_string()));
    }

    #[test]
    fn test_add_relationship() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));
        let relationship = create_test_relationship("src/main.rs", "src/lib.rs");

        matrix.add_relationship(relationship.clone());

        assert_eq!(matrix.relationships.len(), 1);
        let added_rel = &matrix.relationships[0];
        assert_eq!(added_rel.from_file, PathBuf::from("src/main.rs"));
        assert_eq!(added_rel.to_file, PathBuf::from("src/lib.rs"));
        assert_eq!(added_rel.strength, 0.8);
    }

    #[test]
    fn test_add_external_dependency() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));
        let dependency = ExternalDependency {
            name: "serde".to_string(),
            version: Some("1.0.0".to_string()),
            ecosystem: "cargo".to_string(),
            dependency_type: DependencyType::Runtime,
            source_file: PathBuf::from("Cargo.toml"),
        };

        matrix.add_external_dependency(dependency.clone());

        assert_eq!(matrix.external_dependencies.len(), 1);
        let added_dep = &matrix.external_dependencies[0];
        assert_eq!(added_dep.name, "serde");
        assert_eq!(added_dep.version, Some("1.0.0".to_string()));
        assert_eq!(added_dep.ecosystem, "cargo");
    }

    #[test]
    fn test_get_files_by_plugin() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));
        let rust_file1 = create_test_file_node("src/main.rs", "rust");
        let rust_file2 = create_test_file_node("src/lib.rs", "rust");
        let python_file = create_test_file_node("script.py", "python");

        matrix.add_file(rust_file1);
        matrix.add_file(rust_file2);
        matrix.add_file(python_file);

        let rust_files = matrix.get_files_by_plugin("rust");
        let python_files = matrix.get_files_by_plugin("python");
        let missing_files = matrix.get_files_by_plugin("javascript");

        assert_eq!(rust_files.len(), 2);
        assert_eq!(python_files.len(), 1);
        assert_eq!(missing_files.len(), 0);
    }

    #[tokio::test]
    async fn test_save_and_load_matrix() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let matrix_path = temp_dir.path().join("test_matrix.json");

        // Create and populate a matrix
        let mut original_matrix = ProjectMatrix::new(PathBuf::from("/test/project"));
        let file_node = create_test_file_node("src/main.rs", "rust");
        let relationship = create_test_relationship("src/main.rs", "src/lib.rs");

        original_matrix.add_file(file_node);
        original_matrix.add_relationship(relationship);

        // Save the matrix
        original_matrix
            .save(&matrix_path)
            .await
            .expect("Failed to save matrix");

        // Verify the file was created
        assert!(matrix_path.exists());

        // Load the matrix
        let loaded_matrix = ProjectMatrix::load(&matrix_path)
            .await
            .expect("Failed to load matrix");

        // Verify the loaded matrix matches the original
        assert_eq!(
            loaded_matrix.metadata.project_root,
            original_matrix.metadata.project_root
        );
        assert_eq!(
            loaded_matrix.metadata.total_files,
            original_matrix.metadata.total_files
        );
        assert_eq!(loaded_matrix.files.len(), original_matrix.files.len());
        assert_eq!(
            loaded_matrix.relationships.len(),
            original_matrix.relationships.len()
        );

        // Verify specific file exists
        assert!(loaded_matrix
            .files
            .contains_key(&PathBuf::from("src/main.rs")));
    }

    #[test]
    fn test_find_dependencies_and_dependents() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));

        // Create files
        let main_file = create_test_file_node("src/main.rs", "rust");
        let lib_file = create_test_file_node("src/lib.rs", "rust");
        let utils_file = create_test_file_node("src/utils.rs", "rust");

        // Add files to matrix
        matrix.add_file(main_file);
        matrix.add_file(lib_file);
        matrix.add_file(utils_file);

        // Create relationships: main.rs -> lib.rs -> utils.rs
        let rel1 = create_test_relationship("src/main.rs", "src/lib.rs");
        let rel2 = create_test_relationship("src/lib.rs", "src/utils.rs");

        matrix.add_relationship(rel1);
        matrix.add_relationship(rel2);

        // Test dependencies (what main.rs depends on)
        let main_deps = matrix.find_dependencies(&PathBuf::from("src/main.rs"));
        assert_eq!(main_deps.len(), 1);
        assert_eq!(main_deps[0].path, PathBuf::from("src/lib.rs"));

        // Test dependents (what depends on lib.rs)
        let lib_dependents = matrix.find_dependents(&PathBuf::from("src/lib.rs"));
        assert_eq!(lib_dependents.len(), 1);
        assert_eq!(lib_dependents[0].path, PathBuf::from("src/main.rs"));

        // Test file with no dependencies
        let utils_deps = matrix.find_dependencies(&PathBuf::from("src/utils.rs"));
        assert_eq!(utils_deps.len(), 0);

        // Test file with no dependents
        let utils_dependents = matrix.find_dependents(&PathBuf::from("src/utils.rs"));
        assert_eq!(utils_dependents.len(), 1);
        assert_eq!(utils_dependents[0].path, PathBuf::from("src/lib.rs"));
    }

    #[test]
    fn test_calculate_metrics() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));

        // Create a small project structure
        let files = vec![
            create_test_file_node("src/main.rs", "rust"),
            create_test_file_node("src/lib.rs", "rust"),
            create_test_file_node("src/utils.rs", "rust"),
            create_test_file_node("script.py", "python"),
        ];

        for file in files {
            matrix.add_file(file);
        }

        // Add some relationships to make lib.rs highly coupled
        let relationships = vec![
            create_test_relationship("src/main.rs", "src/lib.rs"),
            create_test_relationship("script.py", "src/lib.rs"),
            create_test_relationship("src/utils.rs", "src/lib.rs"),
        ];

        for rel in relationships {
            matrix.add_relationship(rel);
        }

        let metrics = matrix.calculate_metrics();

        assert_eq!(metrics.total_files, 4);
        assert_eq!(metrics.total_relationships, 3);
        assert_eq!(metrics.languages.len(), 2);
        assert!(metrics.languages.contains(&"rust".to_string()));
        assert!(metrics.languages.contains(&"python".to_string()));

        // lib.rs should be the most highly coupled (3 incoming dependencies)
        assert!(!metrics.highly_coupled_files.is_empty());
        assert_eq!(
            metrics.highly_coupled_files[0].0,
            PathBuf::from("src/lib.rs")
        );
        assert_eq!(metrics.highly_coupled_files[0].1, 3); // 3 incoming edges
    }
}
