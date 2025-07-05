// src/core/matrix.rs - Enhanced version with token counting and entrypoint detection
use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{debug, info};
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

    // NEW: Project structure analysis
    pub project_info: ProjectInfo,

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
    pub total_tokens: u64, // NEW: Total estimated tokens across all files
    pub plugins_used: Vec<String>,
}

// NEW: Project-level information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub entrypoints: Vec<EntrypointInfo>,
    pub project_type: ProjectType,
    pub main_language: String,
    pub token_summary: TokenSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrypointInfo {
    pub file_path: PathBuf,
    pub entrypoint_type: String, // "main", "lib", "cli", "web", etc.
    pub confidence: f32,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectType {
    Binary,         // Executable application
    Library,        // Library/package
    WebApplication, // Web server/API
    Mixed,          // Multiple project types
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSummary {
    pub total_tokens: u64,
    pub code_tokens: u64,
    pub documentation_tokens: u64,
    pub average_tokens_per_file: f64,
    pub largest_file_tokens: u64,
    pub largest_file_path: Option<PathBuf>,
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

    // NEW: Token information
    pub token_info: TokenInfo,
}

// NEW: Token information for files and elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub total_tokens: u64,
    pub code_tokens: u64,
    pub documentation_tokens: u64,
    pub comment_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeElement {
    pub element_type: ElementType,
    pub name: String,
    pub signature: Option<String>,
    pub line_start: u32,
    pub line_end: u32,
    pub summary: Option<String>, // Now populated from docstrings/comments
    pub complexity_score: Option<u32>,
    pub calls: Vec<String>,
    pub metadata: serde_json::Value,

    // NEW: Token count for this element
    pub tokens: u64,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub module: String,
    pub items: Vec<String>,
    pub alias: Option<String>,
    pub line_number: u32,
    pub import_type: ImportType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImportType {
    Standard,
    ThirdParty,
    Local,
    Relative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from_file: PathBuf,
    pub to_file: PathBuf,
    pub relationship_type: RelationshipType,
    pub details: String,
    pub line_number: Option<u32>,
    pub strength: f32,
}

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
    pub ecosystem: String,
    pub dependency_type: DependencyType,
    pub source_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

    pub fn add_file(&mut self, file_node: FileNode) {
        debug!("Adding file to matrix: {}", file_node.path.display());

        // Update metadata
        self.metadata.total_files += 1;
        self.metadata.total_size_bytes += file_node.size_bytes;
        self.metadata.total_tokens += file_node.token_info.total_tokens;

        // Update token summary
        self.project_info.token_summary.total_tokens += file_node.token_info.total_tokens;
        self.project_info.token_summary.code_tokens += file_node.token_info.code_tokens;
        self.project_info.token_summary.documentation_tokens +=
            file_node.token_info.documentation_tokens;

        // Track largest file by tokens
        if file_node.token_info.total_tokens > self.project_info.token_summary.largest_file_tokens {
            self.project_info.token_summary.largest_file_tokens = file_node.token_info.total_tokens;
            self.project_info.token_summary.largest_file_path =
                Some(file_node.relative_path.clone());
        }

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

    /// Finalize the matrix after all files are added
    pub fn finalize(&mut self) {
        // Calculate average tokens per file
        if self.metadata.total_files > 0 {
            self.project_info.token_summary.average_tokens_per_file =
                self.project_info.token_summary.total_tokens as f64
                    / self.metadata.total_files as f64;
        }

        // Detect project entrypoints
        self.detect_entrypoints();

        // Determine project type and main language
        self.analyze_project_structure();
    }

    /// Detect project entrypoints based on common patterns
    fn detect_entrypoints(&mut self) {
        let mut entrypoints = Vec::new();

        // Check for Rust entrypoints
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

        // Check for Python entrypoints
        for file in self.files.values() {
            if file.language.as_deref() == Some("python") {
                // Check for __main__.py
                if file.path.file_name().and_then(|n| n.to_str()) == Some("__main__.py") {
                    entrypoints.push(EntrypointInfo {
                        file_path: file.relative_path.clone(),
                        entrypoint_type: "main".to_string(),
                        confidence: 1.0,
                        reason: "Python __main__ module".to_string(),
                    });
                }

                // Check for if __name__ == "__main__": pattern
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

        // Check for web application entrypoints
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

    /// Analyze project structure to determine type and main language
    fn analyze_project_structure(&mut self) {
        // Count files by language
        let mut language_counts: HashMap<String, usize> = HashMap::new();
        for file in self.files.values() {
            if let Some(ref lang) = file.language {
                *language_counts.entry(lang.clone()).or_insert(0) += 1;
            }
        }

        // Determine main language
        if let Some((main_lang, _)) = language_counts.iter().max_by_key(|(_, count)| *count) {
            self.project_info.main_language = main_lang.clone();
        }

        // Determine project type
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

    /// Save the matrix to a JSON file
    pub async fn save(&self, path: &Path) -> Result<()> {
        debug!("Saving project matrix to: {}", path.display());

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let json = serde_json::to_string_pretty(self)?;
        let json_tokens = estimate_tokens(&json);

        // Log the matrix size in tokens
        info!("Matrix JSON size: {json_tokens} tokens");

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

    /// Load a subset of the matrix based on file paths (for token-limited scenarios)
    pub async fn load_subset(path: &Path, file_paths: &[PathBuf]) -> Result<Self> {
        let full_matrix = Self::load(path).await?;
        let mut subset_matrix = ProjectMatrix::new(full_matrix.metadata.project_root.clone());

        // Copy metadata
        subset_matrix.metadata = full_matrix.metadata;
        subset_matrix.project_info = full_matrix.project_info;

        // Copy only requested files
        for file_path in file_paths {
            if let Some(file_node) = full_matrix.files.get(file_path) {
                subset_matrix
                    .files
                    .insert(file_path.clone(), file_node.clone());
            }
        }

        // Copy relationships between included files
        for relationship in &full_matrix.relationships {
            if subset_matrix.files.contains_key(&relationship.from_file)
                && subset_matrix.files.contains_key(&relationship.to_file)
            {
                subset_matrix.relationships.push(relationship.clone());
            }
        }

        // Copy relevant external dependencies
        for dep in &full_matrix.external_dependencies {
            if subset_matrix.files.contains_key(&dep.source_file) {
                subset_matrix.external_dependencies.push(dep.clone());
            }
        }

        subset_matrix.rebuild_graph();
        Ok(subset_matrix)
    }

    /// Get files sorted by token count (useful for prioritizing in LLM context)
    pub fn get_files_by_token_count(&self) -> Vec<(&PathBuf, &FileNode)> {
        let mut files: Vec<_> = self.files.iter().collect();
        files.sort_by_key(|(_, node)| std::cmp::Reverse(node.token_info.total_tokens));
        files
    }

    /// Get a token budget breakdown for LLM context planning
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

        // Create a set for faster lookups
        let included_set: std::collections::HashSet<_> = included_files.iter().cloned().collect();

        // Calculate excluded files
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
            total_tokens: self.metadata.total_tokens,
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

        // Token information
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

        // Entrypoints
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
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudgetInfo {
    pub max_tokens: u64,
    pub used_tokens: u64,
    pub remaining_tokens: u64,
    pub included_files: Vec<PathBuf>,
    pub excluded_files: Vec<PathBuf>,
}

/// Estimate tokens in a string (rough approximation)
/// Uses ~4 characters per token as a heuristic
pub fn estimate_tokens(text: &str) -> u64 {
    (text.len() as f64 / 4.0).ceil() as u64
}

/// Estimate tokens for code more accurately
/// Considers whitespace, punctuation, and common patterns
pub fn estimate_code_tokens(code: &str) -> u64 {
    // Split by whitespace and common delimiters
    let token_count = code
        .split(|c: char| c.is_whitespace() || "(){}[]<>,.;:\"'`|\\/-+=*&%$#@!?~".contains(c))
        .filter(|s| !s.is_empty())
        .count();

    // Add some tokens for the delimiters themselves
    let delimiter_count = code
        .chars()
        .filter(|&c| "(){}[]<>,.;:\"'`|\\/-+=*&%$#@!?~".contains(c))
        .count();

    ((token_count + delimiter_count / 2) as u64).max(1)
}
