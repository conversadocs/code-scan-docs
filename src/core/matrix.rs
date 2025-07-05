//! Project matrix data structures and operations.
//!
//! This module defines the core data model for representing a scanned project,
//! including files, relationships, dependencies, and metadata. The project matrix
//! serves as the central data structure for analysis and documentation generation.

use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{debug, info};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::{Directed, Graph};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Graph representation of project structure using petgraph.
///
/// A directed graph where nodes are files and edges represent relationships
/// between files such as imports, calls, or other dependencies.
pub type ProjectGraph = Graph<FileNode, RelationshipEdge, Directed>;

/// Core data structure representing a complete project analysis.
///
/// Contains all information about a scanned project including files, relationships,
/// external dependencies, and metadata. Provides methods for building, querying,
/// and persisting the project structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMatrix {
    pub metadata: ProjectMetadata,
    pub files: HashMap<PathBuf, FileNode>,
    pub relationships: Vec<Relationship>,
    pub external_dependencies: Vec<ExternalDependency>,

    pub project_info: ProjectInfo,

    #[serde(skip)]
    graph: Option<ProjectGraph>,
    #[serde(skip)]
    node_indexes: HashMap<PathBuf, NodeIndex>,
}

/// Metadata about the project scan operation.
///
/// Contains information about when and how the project was scanned,
/// including timing, version, and summary statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub project_root: PathBuf,
    pub scan_timestamp: DateTime<Utc>,
    pub csd_version: String,
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub total_tokens: u64,
    pub plugins_used: Vec<String>,
}

/// High-level analysis results for the project.
///
/// Contains derived information about the project structure, including
/// detected entrypoints, project type classification, and token analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub entrypoints: Vec<EntrypointInfo>,
    pub project_type: ProjectType,
    pub main_language: String,
    pub token_summary: TokenSummary,
}

/// Information about a detected project entrypoint.
///
/// Represents a file that serves as an entry point to the application,
/// such as main functions, CLI entry points, or web application roots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrypointInfo {
    pub file_path: PathBuf,
    pub entrypoint_type: String,
    pub confidence: f32,
    pub reason: String,
}

/// Classification of the project's primary purpose.
///
/// Categorizes the project based on its structure and detected entrypoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectType {
    Binary,
    Library,
    WebApplication,
    Mixed,
    Unknown,
}

/// Summary of token counts across the project.
///
/// Provides aggregated statistics about code volume measured in tokens,
/// useful for understanding project size and complexity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSummary {
    pub total_tokens: u64,
    pub code_tokens: u64,
    pub documentation_tokens: u64,
    pub average_tokens_per_file: f64,
    pub largest_file_tokens: u64,
    pub largest_file_path: Option<PathBuf>,
}

/// Representation of a single file in the project.
///
/// Contains all analyzed information about a file including its metadata,
/// code elements, dependencies, and token counts.
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

    pub token_info: TokenInfo,
}

/// Token count breakdown for a single file.
///
/// Provides detailed token statistics categorized by content type
/// (code, documentation, comments).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub total_tokens: u64,
    pub code_tokens: u64,
    pub documentation_tokens: u64,
    pub comment_tokens: u64,
}

/// A code element within a file (function, class, etc.).
///
/// Represents a discrete code construct that was identified and analyzed
/// by input plugins, including its location, signature, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeElement {
    pub element_type: ElementType,
    pub name: String,
    pub signature: Option<String>,
    pub line_start: u32,
    pub line_end: u32,
    pub summary: Option<String>,
    pub complexity_score: Option<u32>,
    pub calls: Vec<String>,
    pub metadata: serde_json::Value,

    pub tokens: u64,
}

/// Types of code elements that can be identified in source files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// An import or dependency declaration in a source file.
///
/// Represents how one file depends on another module, library, or file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub module: String,
    pub items: Vec<String>,
    pub alias: Option<String>,
    pub line_number: u32,
    pub import_type: ImportType,
}

/// Classification of import types based on their source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImportType {
    Standard,
    ThirdParty,
    Local,
    Relative,
}

/// A relationship between two files in the project.
///
/// Represents various types of connections between files, such as imports,
/// function calls, or other dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from_file: PathBuf,
    pub to_file: PathBuf,
    pub relationship_type: RelationshipType,
    pub details: String,
    pub line_number: Option<u32>,
    pub strength: f32,
}

/// Types of relationships that can exist between files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RelationshipType {
    Import,
    Call,
    Inheritance,
    Configuration,
    Test,
    Documentation,
    Build,
}

/// Edge data for the project graph representation.
///
/// Contains relationship information used in the graph structure for
/// dependency analysis and visualization.
#[derive(Debug, Clone)]
pub struct RelationshipEdge {
    pub relationship_type: RelationshipType,
    pub strength: f32,
    pub details: String,
}

/// An external dependency used by the project.
///
/// Represents dependencies on libraries, frameworks, or other external
/// code that is not part of the project itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDependency {
    pub name: String,
    pub version: Option<String>,
    pub ecosystem: String,
    pub dependency_type: DependencyType,
    pub source_file: PathBuf,
}

/// Classification of external dependency usage types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyType {
    Runtime,
    Development,
    Build,
    Optional,
}

impl ProjectMatrix {
    /// Creates a new empty project matrix for the given project root.
    ///
    /// Initializes all collections as empty and sets up default metadata
    /// with the current timestamp and version information.
    ///
    /// # Arguments
    ///
    /// * `project_root` - Path to the root directory of the project
    ///
    /// # Returns
    ///
    /// A new empty `ProjectMatrix` ready for population.
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            metadata: ProjectMetadata {
                project_root,
                scan_timestamp: Utc::now(),
                csd_version: env!("CARGO_PKG_VERSION").to_string(),
                total_files: 0,
                total_size_bytes: 0,
                total_tokens: 0,
                plugins_used: Vec::new(),
            },
            files: HashMap::new(),
            relationships: Vec::new(),
            external_dependencies: Vec::new(),
            project_info: ProjectInfo {
                entrypoints: Vec::new(),
                project_type: ProjectType::Unknown,
                main_language: String::new(),
                token_summary: TokenSummary {
                    total_tokens: 0,
                    code_tokens: 0,
                    documentation_tokens: 0,
                    average_tokens_per_file: 0.0,
                    largest_file_tokens: 0,
                    largest_file_path: None,
                },
            },
            graph: None,
            node_indexes: HashMap::new(),
        }
    }

    /// Adds a file node to the matrix and updates aggregate statistics.
    ///
    /// Updates file count, total size, token counts, and plugin usage statistics.
    /// Invalidates the internal graph representation which will be rebuilt on next access.
    ///
    /// # Arguments
    ///
    /// * `file_node` - The file node to add to the matrix
    pub fn add_file(&mut self, file_node: FileNode) {
        debug!("Adding file to matrix: {}", file_node.path.display());

        self.metadata.total_files += 1;
        self.metadata.total_size_bytes += file_node.size_bytes;
        self.metadata.total_tokens += file_node.token_info.total_tokens;

        self.project_info.token_summary.total_tokens += file_node.token_info.total_tokens;
        self.project_info.token_summary.code_tokens += file_node.token_info.code_tokens;
        self.project_info.token_summary.documentation_tokens +=
            file_node.token_info.documentation_tokens;

        if file_node.token_info.total_tokens > self.project_info.token_summary.largest_file_tokens {
            self.project_info.token_summary.largest_file_tokens = file_node.token_info.total_tokens;
            self.project_info.token_summary.largest_file_path =
                Some(file_node.relative_path.clone());
        }

        if !self.metadata.plugins_used.contains(&file_node.plugin) {
            self.metadata.plugins_used.push(file_node.plugin.clone());
        }

        self.files.insert(file_node.path.clone(), file_node);

        self.graph = None;
        self.node_indexes.clear();
    }

    /// Adds a relationship between two files in the matrix.
    ///
    /// Records dependencies, imports, calls, or other connections between files.
    /// Invalidates the internal graph representation.
    ///
    /// # Arguments
    ///
    /// * `relationship` - The relationship to add
    pub fn add_relationship(&mut self, relationship: Relationship) {
        debug!(
            "Adding relationship: {} -> {} ({})",
            relationship.from_file.display(),
            relationship.to_file.display(),
            serde_json::to_string(&relationship.relationship_type).unwrap_or_default()
        );

        self.relationships.push(relationship);

        self.graph = None;
        self.node_indexes.clear();
    }

    /// Adds an external dependency to the matrix.
    ///
    /// Records dependencies on libraries, frameworks, or other external code
    /// that is not part of the project itself.
    ///
    /// # Arguments
    ///
    /// * `dependency` - The external dependency to add
    pub fn add_external_dependency(&mut self, dependency: ExternalDependency) {
        debug!(
            "Adding external dependency: {} from {}",
            dependency.name,
            dependency.source_file.display()
        );
        self.external_dependencies.push(dependency);
    }

    /// Finalizes the matrix after all files and relationships have been added.
    ///
    /// Calculates derived statistics, detects project entrypoints, and analyzes
    /// the overall project structure. Should be called after all scanning is complete.
    pub fn finalize(&mut self) {
        if self.metadata.total_files > 0 {
            self.project_info.token_summary.average_tokens_per_file =
                self.project_info.token_summary.total_tokens as f64
                    / self.metadata.total_files as f64;
        }

        self.detect_entrypoints();

        self.analyze_project_structure();
    }

    /// Detects likely entrypoints for the project based on file patterns and content.
    ///
    /// Looks for common entrypoint patterns like `main.rs`, `lib.rs`, `__main__.py`,
    /// and scripts with main checks. Updates the project info with detected entrypoints.
    fn detect_entrypoints(&mut self) {
        let mut entrypoints = Vec::new();

        if let Some(main_rs) = self
            .files
            .values()
            .find(|f| f.relative_path == PathBuf::from("src/main.rs"))
        {
            entrypoints.push(EntrypointInfo {
                file_path: main_rs.relative_path.clone(),
                entrypoint_type: "cli".to_string(),
                confidence: 1.0,
                reason: "Standard Rust binary entrypoint".to_string(),
            });
        }

        if let Some(lib_rs) = self
            .files
            .values()
            .find(|f| f.relative_path == PathBuf::from("src/lib.rs"))
        {
            entrypoints.push(EntrypointInfo {
                file_path: lib_rs.relative_path.clone(),
                entrypoint_type: "lib".to_string(),
                confidence: 1.0,
                reason: "Standard Rust library entrypoint".to_string(),
            });
        }

        for file in self.files.values() {
            if file.language.as_deref() == Some("python") {
                if file.path.file_name().and_then(|n| n.to_str()) == Some("__main__.py") {
                    entrypoints.push(EntrypointInfo {
                        file_path: file.relative_path.clone(),
                        entrypoint_type: "main".to_string(),
                        confidence: 1.0,
                        reason: "Python __main__ module".to_string(),
                    });
                }

                for element in &file.elements {
                    if element.element_type == ElementType::Variable
                        && element.name == "__name__"
                        && element
                            .metadata
                            .get("is_main_check")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                    {
                        entrypoints.push(EntrypointInfo {
                            file_path: file.relative_path.clone(),
                            entrypoint_type: "script".to_string(),
                            confidence: 0.9,
                            reason: "Python script with main check".to_string(),
                        });
                    }
                }
            }
        }

        if self.files.values().any(|f| {
            f.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| {
                    n == "app.py"
                        || n == "main.py"
                        || n == "server.py"
                        || n == "index.js"
                        || n == "app.js"
                })
                .unwrap_or(false)
        }) {
            // Web framework detection would go here
        }

        self.project_info.entrypoints = entrypoints;
    }

    /// Analyzes the project structure to determine project type and main language.
    ///
    /// Examines file languages and entrypoints to classify the project as a binary,
    /// library, mixed project, or web application.
    fn analyze_project_structure(&mut self) {
        let mut language_counts: HashMap<String, usize> = HashMap::new();
        for file in self.files.values() {
            if let Some(ref lang) = file.language {
                *language_counts.entry(lang.clone()).or_insert(0) += 1;
            }
        }

        if let Some((main_lang, _)) = language_counts.iter().max_by_key(|(_, count)| *count) {
            self.project_info.main_language = main_lang.clone();
        }

        let has_main = self
            .project_info
            .entrypoints
            .iter()
            .any(|e| e.entrypoint_type == "cli" || e.entrypoint_type == "main");
        let has_lib = self
            .project_info
            .entrypoints
            .iter()
            .any(|e| e.entrypoint_type == "lib");

        self.project_info.project_type = match (has_main, has_lib) {
            (true, true) => ProjectType::Mixed,
            (true, false) => ProjectType::Binary,
            (false, true) => ProjectType::Library,
            _ => ProjectType::Unknown,
        };
    }

    /// Saves the project matrix to a JSON file.
    ///
    /// Serializes the complete matrix to JSON format and writes it to the specified path.
    /// Creates parent directories if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the matrix file should be saved
    ///
    /// # Returns
    ///
    /// Result indicating success or failure of the save operation.
    pub async fn save(&self, path: &Path) -> Result<()> {
        debug!("Saving project matrix to: {}", path.display());

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string_pretty(self)?;
        let json_tokens = estimate_tokens(&json);

        info!("Matrix JSON size: {json_tokens} tokens");

        tokio::fs::write(path, json).await?;

        debug!("Matrix saved successfully");
        Ok(())
    }

    /// Loads a project matrix from a JSON file.
    ///
    /// Deserializes a previously saved matrix and rebuilds the internal graph structure.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the matrix file to load
    ///
    /// # Returns
    ///
    /// The loaded project matrix or an error if loading fails.
    pub async fn load(path: &Path) -> Result<Self> {
        debug!("Loading project matrix from: {}", path.display());

        let json = tokio::fs::read_to_string(path).await?;
        let mut matrix: ProjectMatrix = serde_json::from_str(&json)?;

        matrix.rebuild_graph();

        debug!(
            "Matrix loaded successfully with {} files",
            matrix.files.len()
        );
        Ok(matrix)
    }

    /// Loads a subset of files from a saved project matrix.
    ///
    /// Creates a new matrix containing only the specified files and their
    /// relationships, useful for focused analysis of part of a project.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the full matrix file
    /// * `file_paths` - Paths of files to include in the subset
    ///
    /// # Returns
    ///
    /// A new matrix containing only the specified files and their relationships.
    pub async fn load_subset(path: &Path, file_paths: &[PathBuf]) -> Result<Self> {
        let full_matrix = Self::load(path).await?;
        let mut subset_matrix = ProjectMatrix::new(full_matrix.metadata.project_root.clone());

        subset_matrix.metadata = full_matrix.metadata;
        subset_matrix.project_info = full_matrix.project_info;

        for file_path in file_paths {
            if let Some(file_node) = full_matrix.files.get(file_path) {
                subset_matrix
                    .files
                    .insert(file_path.clone(), file_node.clone());
            }
        }

        for relationship in &full_matrix.relationships {
            if subset_matrix.files.contains_key(&relationship.from_file)
                && subset_matrix.files.contains_key(&relationship.to_file)
            {
                subset_matrix.relationships.push(relationship.clone());
            }
        }

        for dep in &full_matrix.external_dependencies {
            if subset_matrix.files.contains_key(&dep.source_file) {
                subset_matrix.external_dependencies.push(dep.clone());
            }
        }

        subset_matrix.rebuild_graph();
        Ok(subset_matrix)
    }

    /// Returns files sorted by token count in descending order.
    ///
    /// Useful for identifying the largest files in the project or for
    /// token budget allocation in analysis operations.
    ///
    /// # Returns
    ///
    /// Vector of (path, file_node) tuples sorted by total token count.
    pub fn get_files_by_token_count(&self) -> Vec<(&PathBuf, &FileNode)> {
        let mut files: Vec<_> = self.files.iter().collect();
        files.sort_by_key(|(_, node)| std::cmp::Reverse(node.token_info.total_tokens));
        files
    }

    /// Calculates which files can fit within a given token budget.
    ///
    /// Determines the optimal set of files to include when there's a limit
    /// on total tokens that can be processed, prioritizing larger files first.
    ///
    /// # Arguments
    ///
    /// * `max_tokens` - Maximum number of tokens available
    ///
    /// # Returns
    ///
    /// Information about which files are included/excluded and token usage.
    pub fn get_token_budget_info(&self, max_tokens: u64) -> TokenBudgetInfo {
        let mut included_files = Vec::new();
        let mut remaining_tokens = max_tokens;
        let mut total_included_tokens = 0;

        for (path, file) in self.get_files_by_token_count() {
            if file.token_info.total_tokens <= remaining_tokens {
                included_files.push(path.clone());
                total_included_tokens += file.token_info.total_tokens;
                remaining_tokens -= file.token_info.total_tokens;
            }
        }

        let included_set: std::collections::HashSet<_> = included_files.iter().cloned().collect();

        let excluded_files: Vec<PathBuf> = self
            .files
            .keys()
            .filter(|k| !included_set.contains(*k))
            .cloned()
            .collect();

        TokenBudgetInfo {
            max_tokens,
            used_tokens: total_included_tokens,
            remaining_tokens,
            included_files,
            excluded_files,
        }
    }

    /// Rebuilds the internal graph representation from matrix data.
    ///
    /// Creates a new petgraph structure with files as nodes and relationships
    /// as edges. Called automatically when the graph is needed after modifications.
    fn rebuild_graph(&mut self) {
        debug!("Rebuilding graph from matrix data");

        let mut graph = Graph::new();
        let mut node_indexes = HashMap::new();

        for (path, file_node) in &self.files {
            let node_index = graph.add_node(file_node.clone());
            node_indexes.insert(path.clone(), node_index);
        }

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

    /// Ensures the internal graph representation is built and up-to-date.
    ///
    /// Rebuilds the graph if it has been invalidated by adding files or relationships.
    fn ensure_graph(&mut self) {
        if self.graph.is_none() {
            self.rebuild_graph();
        }
    }

    /// Finds all files that depend on the specified file.
    ///
    /// Returns files that import from, call into, or otherwise depend on
    /// the given file.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path of the file to find dependents for
    ///
    /// # Returns
    ///
    /// Vector of file nodes that depend on the specified file.
    pub fn find_dependents(&mut self, file_path: &Path) -> Vec<&FileNode> {
        self.ensure_graph();

        let graph = self.graph.as_ref().unwrap();
        let mut dependents = Vec::new();

        if let Some(&node_idx) = self.node_indexes.get(file_path) {
            for edge_ref in graph.edges_directed(node_idx, petgraph::Direction::Incoming) {
                let dependent_idx = edge_ref.source();
                if let Some(file_node) = graph.node_weight(dependent_idx) {
                    dependents.push(file_node);
                }
            }
        }

        dependents
    }

    /// Finds all files that the specified file depends on.
    ///
    /// Returns files that are imported by, called by, or otherwise depended
    /// upon by the given file.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path of the file to find dependencies for
    ///
    /// # Returns
    ///
    /// Vector of file nodes that the specified file depends on.
    pub fn find_dependencies(&mut self, file_path: &Path) -> Vec<&FileNode> {
        self.ensure_graph();

        let graph = self.graph.as_ref().unwrap();
        let mut dependencies = Vec::new();

        if let Some(&node_idx) = self.node_indexes.get(file_path) {
            for edge_ref in graph.edges_directed(node_idx, petgraph::Direction::Outgoing) {
                let dependency_idx = edge_ref.target();
                if let Some(file_node) = graph.node_weight(dependency_idx) {
                    dependencies.push(file_node);
                }
            }
        }

        dependencies
    }

    /// Returns all files that were processed by a specific plugin.
    ///
    /// Useful for understanding which plugin handled which files and for
    /// plugin-specific analysis.
    ///
    /// # Arguments
    ///
    /// * `plugin_name` - Name of the plugin to filter by
    ///
    /// # Returns
    ///
    /// Vector of file nodes processed by the specified plugin.
    pub fn get_files_by_plugin(&self, plugin_name: &str) -> Vec<&FileNode> {
        self.files
            .values()
            .filter(|file| file.plugin == plugin_name)
            .collect()
    }

    /// Calculates various metrics about the project structure.
    ///
    /// Analyzes the project graph to determine coupling scores, identify
    /// highly connected files, and provide other structural insights.
    ///
    /// # Returns
    ///
    /// Calculated metrics including coupling analysis and summary statistics.
    pub fn calculate_metrics(&mut self) -> ProjectMetrics {
        self.ensure_graph();

        let graph = self.graph.as_ref().unwrap();

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
            total_tokens: self.metadata.total_tokens,
        }
    }

    /// Prints a human-readable summary of the project matrix to stdout.
    ///
    /// Displays key statistics, file breakdown by plugin, token information,
    /// detected entrypoints, and other useful summary information.
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

        println!("\n📊 Token Summary:");
        println!(
            "  Total tokens: {}",
            self.project_info.token_summary.total_tokens
        );
        println!(
            "  Code tokens: {}",
            self.project_info.token_summary.code_tokens
        );
        println!(
            "  Documentation tokens: {}",
            self.project_info.token_summary.documentation_tokens
        );
        println!(
            "  Average per file: {:.0}",
            self.project_info.token_summary.average_tokens_per_file
        );
        if let Some(ref largest_file) = self.project_info.token_summary.largest_file_path {
            println!(
                "  Largest file: {} ({} tokens)",
                largest_file.display(),
                self.project_info.token_summary.largest_file_tokens
            );
        }

        if !self.project_info.entrypoints.is_empty() {
            println!("\n🚀 Detected Entrypoints:");
            for entry in &self.project_info.entrypoints {
                println!(
                    "  {} ({}, confidence: {:.0}%)",
                    entry.file_path.display(),
                    entry.entrypoint_type,
                    entry.confidence * 100.0
                );
            }
        }

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

            let mut sorted_files = files_for_plugin.clone();
            sorted_files.sort();
            for (i, file_path) in sorted_files.iter().enumerate() {
                if i < 5 {
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

/// Calculated metrics about the project structure.
///
/// Provides analysis results including coupling scores, complexity measures,
/// and other derived insights about the codebase.
#[derive(Debug)]
pub struct ProjectMetrics {
    pub total_files: usize,
    pub total_relationships: usize,
    pub highly_coupled_files: Vec<(PathBuf, usize)>,
    pub languages: Vec<String>,
    pub total_tokens: u64,
}

/// Information about token budget allocation for analysis.
///
/// Helps determine which files can be included in analysis operations
/// that have token limits, such as LLM processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudgetInfo {
    pub max_tokens: u64,
    pub used_tokens: u64,
    pub remaining_tokens: u64,
    pub included_files: Vec<PathBuf>,
    pub excluded_files: Vec<PathBuf>,
}

/// Estimates the number of tokens in a text string.
///
/// Uses a simple heuristic of approximately 4 characters per token,
/// which works reasonably well for most text content.
///
/// # Arguments
///
/// * `text` - The text to analyze
///
/// # Returns
///
/// Estimated number of tokens in the text.
pub fn estimate_tokens(text: &str) -> u64 {
    (text.len() as f64 / 4.0).ceil() as u64
}

/// Estimates the number of tokens in source code.
///
/// Uses a more sophisticated approach than general text, counting
/// identifiers, keywords, and delimiters separately for better
/// accuracy with code content.
///
/// # Arguments
///
/// * `code` - The source code to analyze
///
/// # Returns
///
/// Estimated number of tokens in the code.
pub fn estimate_code_tokens(code: &str) -> u64 {
    let token_count = code
        .split(|c: char| c.is_whitespace() || "(){}[]<>,.;:\"'`|\\/-+=*&%$#@!?~".contains(c))
        .filter(|s| !s.is_empty())
        .count();

    let delimiter_count = code
        .chars()
        .filter(|&c| "(){}[]<>,.;:\"'`|\\/-+=*&%$#@!?~".contains(c))
        .count();

    ((token_count + delimiter_count / 2) as u64).max(1)
}
