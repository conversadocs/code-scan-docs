use anyhow::Result;
use chrono::{DateTime, Utc};
use petgraph::{Graph, Directed};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use log::{info, debug};

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
    pub calls: Vec<String>, // Functions/methods this element calls
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
    Standard,    // from standard library
    ThirdParty,  // from external dependency
    Local,       // from project files
    Relative,    // relative import
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
    Call,         // Function call across files
    Inheritance,  // Class inheritance
    Configuration,// Config file affects code file
    Test,         // Test file tests source file
    Documentation,// Doc file documents source file
    Build,        // Build file affects source file
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
        debug!("Adding relationship: {} -> {} ({})",
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
        debug!("Adding external dependency: {} from {}",
            dependency.name, dependency.source_file.display());
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

        debug!("Matrix loaded successfully with {} files", matrix.files.len());
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
                node_indexes.get(&relationship.to_file)
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

        debug!("Graph rebuilt with {} nodes and {} edges",
            self.files.len(), self.relationships.len());
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
        self.files.values()
            .filter(|file| file.plugin == plugin_name)
            .collect()
    }

    /// Calculate some basic metrics
    pub fn calculate_metrics(&mut self) -> ProjectMetrics {
        self.ensure_graph();

        let graph = self.graph.as_ref().unwrap();

        // Find files with highest in-degree (most depended upon)
        let mut coupling_scores: Vec<(PathBuf, usize)> = self.node_indexes.iter()
            .map(|(path, &idx)| {
                let in_degree = graph.edges_directed(idx, petgraph::Direction::Incoming).count();
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
        println!("Scanned: {}", self.metadata.scan_timestamp.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("Files: {}", self.metadata.total_files);
        println!("Total size: {:.2} MB", self.metadata.total_size_bytes as f64 / (1024.0 * 1024.0));
        println!("Relationships: {}", self.relationships.len());
        println!("External dependencies: {}", self.external_dependencies.len());
        println!("Languages: {}", self.metadata.plugins_used.join(", "));

        // Show files scanned by language/plugin
        println!("\n📁 Files scanned:");
        let mut by_plugin: std::collections::HashMap<String, Vec<&PathBuf>> =
            std::collections::HashMap::new();

        for (path, file_node) in &self.files {
            by_plugin.entry(file_node.plugin.clone()).or_default().push(path);
        }

        let mut plugins: Vec<_> = by_plugin.keys().collect();
        plugins.sort();

        for plugin in plugins {
            let files_for_plugin = &by_plugin[plugin];
            println!("  {} ({} files)", plugin.to_uppercase(), files_for_plugin.len());

            // Show first few files for each plugin
            let mut sorted_files = files_for_plugin.clone();
            sorted_files.sort();
            for (i, file_path) in sorted_files.iter().enumerate() {
                if i < 5 {  // Show first 5 files
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
