//! Project scanning and file analysis functionality.
//!
//! This module provides the core scanning engine that walks through project directories,
//! identifies files, and coordinates with input plugins to analyze code and build
//! the project matrix.

use crate::core::matrix::{estimate_code_tokens, estimate_tokens, ProjectMatrix, TokenInfo};
use crate::plugins::interface::{InputPluginInterface, PluginInput};
use crate::utils::config::Config;
use anyhow::Result;
use ignore::WalkBuilder;
use log::{debug, info, warn};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Information about a discovered file during project scanning.
///
/// Contains metadata about a file including its path, size, content type,
/// and which plugin should handle its analysis.
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub extension: Option<String>,
    pub size_bytes: u64,
    pub is_text: bool,
    pub plugin_name: Option<String>,
    pub content_hash: String,
}

/// Main project scanner that coordinates file discovery and analysis.
///
/// Walks through project directories, identifies relevant files, and uses
/// input plugins to analyze code structure and relationships.
pub struct ProjectScanner {
    config: Config,
    project_root: PathBuf,
}

impl ProjectScanner {
    /// Creates a new project scanner with the given configuration.
    ///
    /// The scanner will use the current directory as the project root by default.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration containing plugin settings and scanning options
    ///
    /// # Returns
    ///
    /// A new `ProjectScanner` instance ready for scanning.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            project_root: PathBuf::from("."),
        }
    }

    /// Sets the project root directory for scanning.
    ///
    /// # Arguments
    ///
    /// * `root` - Path to the project root directory
    ///
    /// # Returns
    ///
    /// The scanner with the updated root path.
    pub fn with_root<P: AsRef<Path>>(mut self, root: P) -> Self {
        self.project_root = root.as_ref().to_path_buf();
        self
    }

    /// Performs a complete project scan and builds the project matrix.
    ///
    /// Discovers all relevant files, analyzes them with appropriate plugins,
    /// and constructs a comprehensive project matrix with relationships
    /// and metadata.
    ///
    /// # Returns
    ///
    /// A complete project matrix or an error if scanning fails.
    pub async fn scan_to_matrix(&self) -> Result<ProjectMatrix> {
        debug!(
            "Starting file scan and matrix creation in: {}",
            self.project_root.display()
        );

        let mut matrix = ProjectMatrix::new(self.project_root.clone());
        let files = self.scan().await?;

        debug!("Found {} files, analyzing with plugins...", files.len());

        for file_info in files {
            debug!(
                "🔍 Processing file: {} (is_text: {}, plugin: {:?})",
                file_info.path.display(),
                file_info.is_text,
                file_info.plugin_name
            );

            let file_node = if file_info.is_text && file_info.plugin_name.is_some() {
                debug!("✅ Calling plugin for: {}", file_info.path.display());
                self.analyze_file_with_plugin(&file_info, &mut matrix)
                    .await?
            } else {
                debug!(
                    "❌ Skipping plugin for: {} (is_text: {}, plugin: {:?})",
                    file_info.path.display(),
                    file_info.is_text,
                    file_info.plugin_name
                );
                self.create_basic_file_node(&file_info).await?
            };

            matrix.add_file(file_node);
        }

        matrix.finalize();

        debug!("Matrix created with {} files", matrix.files.len());
        Ok(matrix)
    }

    /// Analyzes a single file using its designated input plugin.
    ///
    /// Loads the file content, communicates with the appropriate plugin,
    /// and converts the plugin output into matrix data structures.
    ///
    /// # Arguments
    ///
    /// * `file_info` - Information about the file to analyze
    /// * `matrix` - Project matrix to add relationships and dependencies to
    ///
    /// # Returns
    ///
    /// A file node with analysis results or an error if analysis fails.
    async fn analyze_file_with_plugin(
        &self,
        file_info: &FileInfo,
        matrix: &mut ProjectMatrix,
    ) -> Result<crate::core::matrix::FileNode> {
        info!("🚀 Starting analysis for: {}", file_info.path.display());

        use crate::plugins::communication::InputPluginCommunicator;

        let plugin_name = file_info.plugin_name.as_ref().unwrap();
        debug!("📝 Plugin name: {plugin_name}");

        let plugin_config = self
            .config
            .get_input_plugin(plugin_name)
            .ok_or_else(|| anyhow::anyhow!("Input plugin {} not found in config", plugin_name))?;

        debug!("⚙️ Got input plugin config for: {plugin_name}");

        let plugin_path = match &plugin_config.source {
            crate::utils::config::PluginSource::Builtin { name, plugin_type } => {
                PathBuf::from(format!("plugins/input/{plugin_type}/{name}.py"))
            }
            crate::utils::config::PluginSource::Local { path } => PathBuf::from(path),
            _ => {
                // TODO: Handle other plugin sources (GitHub, Git)
                return self.create_basic_file_node(file_info).await;
            }
        };

        debug!("📂 Plugin path resolved to: {}", plugin_path.display());

        if !plugin_path.exists() {
            warn!("Plugin file not found: {}", plugin_path.display());
            return self.create_basic_file_node(file_info).await;
        }

        debug!("✅ Plugin file exists");

        debug!("📖 Reading file content...");
        let content = match tokio::fs::read_to_string(&file_info.path).await {
            Ok(content) => {
                debug!("✅ File content read ({} bytes)", content.len());
                content
            }
            Err(e) => {
                warn!("Could not read file {}: {}", file_info.path.display(), e);
                return self.create_basic_file_node(file_info).await;
            }
        };

        let cache_dir = self.project_root.join(".csd_cache");

        debug!("🔧 Creating plugin input...");
        let plugin_input = PluginInput {
            file_path: file_info.path.clone(),
            relative_path: file_info.relative_path.clone(),
            content,
            project_root: self.project_root.clone(),
            cache_dir: cache_dir.to_string_lossy().to_string(),
            plugin_config: plugin_config
                .config
                .as_ref()
                .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
        };

        debug!("📡 Creating plugin communicator...");
        let mut communicator = InputPluginCommunicator::new(plugin_path).with_cache_dir(cache_dir);

        if let Some(ref python_exe) = self.config.python_executable {
            communicator = communicator.with_python_executable(python_exe.clone());
        } else {
            communicator = communicator.with_python_auto_detect();
        }

        debug!("🔄 Starting plugin communication...");
        match communicator.analyze(plugin_input).await {
            Ok(plugin_output) => {
                info!(
                    "✅ Analysis successful for: {} with {} elements",
                    file_info.path.display(),
                    plugin_output.elements.len()
                );

                self.convert_plugin_output_to_file_node(file_info, plugin_output, matrix)
                    .await
            }
            Err(e) => {
                warn!(
                    "❌ Plugin analysis failed for {}: {}",
                    file_info.path.display(),
                    e
                );
                self.create_basic_file_node(file_info).await
            }
        }
    }

    /// Converts plugin analysis output into matrix data structures.
    ///
    /// Transforms plugin-specific output formats into the standard matrix
    /// format and adds relationships and dependencies to the project matrix.
    ///
    /// # Arguments
    ///
    /// * `file_info` - Original file information
    /// * `plugin_output` - Analysis results from the input plugin
    /// * `matrix` - Project matrix to update with relationships
    ///
    /// # Returns
    ///
    /// A file node containing the converted analysis results.
    async fn convert_plugin_output_to_file_node(
        &self,
        file_info: &FileInfo,
        plugin_output: crate::plugins::interface::PluginOutput,
        matrix: &mut ProjectMatrix,
    ) -> Result<crate::core::matrix::FileNode> {
        use crate::core::matrix::{ExternalDependency, Relationship};

        let elements: Vec<crate::core::matrix::CodeElement> = plugin_output
            .elements
            .into_iter()
            .map(|e| {
                let summary = e.summary.or_else(|| {
                    e.metadata
                        .get("docstring")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                });

                crate::core::matrix::CodeElement {
                    element_type: match e.element_type.as_str() {
                        "function" => crate::core::matrix::ElementType::Function,
                        "method" => crate::core::matrix::ElementType::Method,
                        "class" => crate::core::matrix::ElementType::Class,
                        "struct" => crate::core::matrix::ElementType::Struct,
                        "enum" => crate::core::matrix::ElementType::Enum,
                        "interface" => crate::core::matrix::ElementType::Interface,
                        "module" => crate::core::matrix::ElementType::Module,
                        "variable" => crate::core::matrix::ElementType::Variable,
                        "constant" => crate::core::matrix::ElementType::Constant,
                        "type" => crate::core::matrix::ElementType::Type,
                        _ => crate::core::matrix::ElementType::Function,
                    },
                    name: e.name,
                    signature: e.signature,
                    line_start: e.line_start,
                    line_end: e.line_end,
                    summary,
                    complexity_score: e.complexity_score,
                    calls: e.calls,
                    metadata: e.metadata,
                    tokens: e.tokens.unwrap_or(0),
                }
            })
            .collect();

        let imports: Vec<crate::core::matrix::Import> = plugin_output
            .imports
            .into_iter()
            .map(|i| crate::core::matrix::Import {
                module: i.module,
                items: i.items,
                alias: i.alias,
                line_number: i.line_number,
                import_type: match i.import_type.as_str() {
                    "standard" => crate::core::matrix::ImportType::Standard,
                    "third_party" => crate::core::matrix::ImportType::ThirdParty,
                    "local" => crate::core::matrix::ImportType::Local,
                    "relative" => crate::core::matrix::ImportType::Relative,
                    _ => crate::core::matrix::ImportType::Standard,
                },
            })
            .collect();

        for rel in plugin_output.relationships {
            let relationship = Relationship {
                from_file: PathBuf::from(rel.from_file),
                to_file: PathBuf::from(rel.to_file),
                relationship_type: match rel.relationship_type.as_str() {
                    "import" => crate::core::matrix::RelationshipType::Import,
                    "call" => crate::core::matrix::RelationshipType::Call,
                    "inheritance" => crate::core::matrix::RelationshipType::Inheritance,
                    "configuration" => crate::core::matrix::RelationshipType::Configuration,
                    "test" => crate::core::matrix::RelationshipType::Test,
                    "documentation" => crate::core::matrix::RelationshipType::Documentation,
                    "build" => crate::core::matrix::RelationshipType::Build,
                    _ => crate::core::matrix::RelationshipType::Import,
                },
                details: rel.details,
                line_number: rel.line_number,
                strength: rel.strength,
            };
            matrix.add_relationship(relationship);
        }

        for dep in plugin_output.external_dependencies {
            let dependency = ExternalDependency {
                name: dep.name,
                version: dep.version,
                ecosystem: dep.ecosystem,
                dependency_type: match dep.dependency_type.as_str() {
                    "runtime" => crate::core::matrix::DependencyType::Runtime,
                    "development" => crate::core::matrix::DependencyType::Development,
                    "build" => crate::core::matrix::DependencyType::Build,
                    "optional" => crate::core::matrix::DependencyType::Optional,
                    _ => crate::core::matrix::DependencyType::Runtime,
                },
                source_file: PathBuf::from(dep.source_file),
            };
            matrix.add_external_dependency(dependency);
        }

        let token_info = if let Some(token_info_value) = plugin_output.token_info {
            if let Ok(token_map) =
                serde_json::from_value::<std::collections::HashMap<String, u64>>(token_info_value)
            {
                TokenInfo {
                    total_tokens: token_map.get("total_tokens").copied().unwrap_or(0),
                    code_tokens: token_map.get("code_tokens").copied().unwrap_or(0),
                    documentation_tokens: token_map
                        .get("documentation_tokens")
                        .copied()
                        .unwrap_or(0),
                    comment_tokens: token_map.get("comment_tokens").copied().unwrap_or(0),
                }
            } else {
                let estimated_tokens =
                    estimate_tokens(&std::fs::read_to_string(&file_info.path).unwrap_or_default());
                TokenInfo {
                    total_tokens: estimated_tokens,
                    code_tokens: estimated_tokens,
                    documentation_tokens: 0,
                    comment_tokens: 0,
                }
            }
        } else {
            let estimated_tokens =
                estimate_tokens(&std::fs::read_to_string(&file_info.path).unwrap_or_default());
            TokenInfo {
                total_tokens: estimated_tokens,
                code_tokens: estimated_tokens,
                documentation_tokens: 0,
                comment_tokens: 0,
            }
        };

        Ok(crate::core::matrix::FileNode {
            path: file_info.path.clone(),
            relative_path: file_info.relative_path.clone(),
            hash: file_info.content_hash.clone(),
            size_bytes: file_info.size_bytes,
            plugin: file_info
                .plugin_name
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            language: self.config.find_input_plugin_for_file(&file_info.path),
            is_text: file_info.is_text,
            elements,
            imports,
            exports: plugin_output.exports,
            file_summary: plugin_output.file_summary,
            token_info,
        })
    }

    /// Creates a basic file node without plugin analysis.
    ///
    /// Used for files that don't have associated plugins or when plugin
    /// analysis fails. Provides basic metadata and token estimates.
    ///
    /// # Arguments
    ///
    /// * `file_info` - Information about the file
    ///
    /// # Returns
    ///
    /// A basic file node with minimal analysis data.
    async fn create_basic_file_node(
        &self,
        file_info: &FileInfo,
    ) -> Result<crate::core::matrix::FileNode> {
        let token_info = if file_info.is_text {
            match tokio::fs::read_to_string(&file_info.path).await {
                Ok(content) => {
                    let total_tokens = estimate_code_tokens(&content);
                    TokenInfo {
                        total_tokens,
                        code_tokens: total_tokens,
                        documentation_tokens: 0,
                        comment_tokens: 0,
                    }
                }
                Err(_) => TokenInfo {
                    total_tokens: 0,
                    code_tokens: 0,
                    documentation_tokens: 0,
                    comment_tokens: 0,
                },
            }
        } else {
            TokenInfo {
                total_tokens: 0,
                code_tokens: 0,
                documentation_tokens: 0,
                comment_tokens: 0,
            }
        };

        Ok(crate::core::matrix::FileNode {
            path: file_info.path.clone(),
            relative_path: file_info.relative_path.clone(),
            hash: file_info.content_hash.clone(),
            size_bytes: file_info.size_bytes,
            plugin: file_info
                .plugin_name
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            language: self.config.find_input_plugin_for_file(&file_info.path),
            is_text: file_info.is_text,
            elements: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            file_summary: None,
            token_info,
        })
    }

    /// Discovers all relevant files in the project directory.
    ///
    /// Walks through the project directory structure, applies filtering rules,
    /// and identifies files that should be included in the analysis.
    ///
    /// # Returns
    ///
    /// A vector of file information for all discovered files.
    pub async fn scan(&self) -> Result<Vec<FileInfo>> {
        debug!("Starting file scan in: {}", self.project_root.display());

        let mut files = Vec::new();
        let mut _total_files = 0;
        let mut skipped_files = 0;

        let walker = WalkBuilder::new(&self.project_root)
            .hidden(!self.config.scanning.include_hidden)
            .git_ignore(true)
            .git_exclude(true)
            .build();

        for entry in walker {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    warn!("Error reading directory entry: {e}");
                    continue;
                }
            };

            _total_files += 1;

            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                continue;
            }

            let path = entry.path();

            if self.should_ignore_file(path) {
                debug!("Ignoring file: {}", path.display());
                skipped_files += 1;
                continue;
            }

            let metadata = match std::fs::metadata(path) {
                Ok(metadata) => metadata,
                Err(e) => {
                    warn!("Could not read metadata for {}: {}", path.display(), e);
                    skipped_files += 1;
                    continue;
                }
            };

            let size_bytes = metadata.len();
            let max_size = self.config.scanning.max_file_size_mb * 1024 * 1024;

            if size_bytes > max_size {
                debug!(
                    "File too large, skipping: {} ({} bytes)",
                    path.display(),
                    size_bytes
                );
                skipped_files += 1;
                continue;
            }

            let relative_path = match path.strip_prefix(&self.project_root) {
                Ok(rel) => rel.to_path_buf(),
                Err(_) => path.to_path_buf(),
            };

            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| format!(".{}", ext.to_lowercase()));

            let is_text = self.is_text_file(path, &extension);
            let plugin_name = self.config.find_input_plugin_for_file(path);

            let content_hash = self
                .calculate_file_hash(path)
                .unwrap_or_else(|_| "error".to_string());

            let file_info = FileInfo {
                path: path.to_path_buf(),
                relative_path,
                extension,
                size_bytes,
                is_text,
                plugin_name,
                content_hash,
            };

            debug!("Found file: {file_info:?}");
            files.push(file_info);
        }

        debug!(
            "Scan complete. Found {} files, skipped {} files",
            files.len(),
            skipped_files
        );

        Ok(files)
    }

    /// Calculates a SHA-256 hash of the file content.
    ///
    /// Used for change detection and caching purposes.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to hash
    ///
    /// # Returns
    ///
    /// Hexadecimal string representation of the file hash.
    fn calculate_file_hash(&self, path: &Path) -> Result<String> {
        let content = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        Ok(format!("{hash:x}"))
    }

    /// Determines if a file should be ignored based on configuration patterns.
    ///
    /// Checks the file path against configured ignore patterns including
    /// directories, extensions, and partial path matches.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to check against ignore patterns
    ///
    /// # Returns
    ///
    /// `true` if the file should be ignored, `false` otherwise.
    fn should_ignore_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.config.scanning.ignore_patterns {
            if pattern.ends_with('/') {
                let dir_pattern = &pattern[..pattern.len() - 1];
                if path_str.contains(dir_pattern) {
                    return true;
                }
            } else if pattern.starts_with("*.") {
                let ext = &pattern[1..];
                if path_str.ends_with(ext) {
                    return true;
                }
            } else if path_str.contains(pattern) {
                return true;
            }
        }

        false
    }

    /// Determines if a file contains text content that should be analyzed.
    ///
    /// Checks file extensions and names against known text file patterns
    /// and plugin associations.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `extension` - File extension (if any)
    ///
    /// # Returns
    ///
    /// `true` if the file is identified as containing text content.
    fn is_text_file(&self, path: &Path, extension: &Option<String>) -> bool {
        if self.config.find_input_plugin_for_file(path).is_some() {
            return true;
        }

        if let Some(ext) = extension {
            matches!(
                ext.as_str(),
                ".md"
                    | ".rst"
                    | ".txt"
                    | ".asciidoc"
                    | ".adoc"
                    | ".org"
                    | ".tex"
                    | ".ini"
                    | ".cfg"
                    | ".conf"
                    | ".properties"
                    | ".env"
                    | ".gitignore"
                    | ".gitattributes"
                    | ".dockerignore"
                    | ".editorconfig"
            )
        } else {
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            matches!(
                filename.to_lowercase().as_str(),
                "readme"
                    | "license"
                    | "copyright"
                    | "authors"
                    | "contributors"
                    | "changelog"
                    | "news"
                    | "dockerfile"
                    | "makefile"
                    | ".gitignore"
                    | ".gitattributes"
                    | ".dockerignore"
                    | ".editorconfig"
            )
        }
    }

    /// Prints a human-readable summary of the file scan results.
    ///
    /// Displays statistics about discovered files, plugin associations,
    /// and overall project structure.
    ///
    /// # Arguments
    ///
    /// * `files` - Vector of discovered file information to summarize
    pub fn print_scan_results(&self, files: &[FileInfo]) {
        println!("\n=== CSD File Scan Results ===");
        println!("Project root: {}", self.project_root.display());
        println!("Total files found: {}\n", files.len());

        let mut by_plugin: std::collections::HashMap<String, Vec<&FileInfo>> =
            std::collections::HashMap::new();
        let mut unknown_files = Vec::new();

        for file in files {
            match &file.plugin_name {
                Some(plugin) => {
                    by_plugin.entry(plugin.clone()).or_default().push(file);
                }
                None => {
                    unknown_files.push(file);
                }
            }
        }

        let mut plugins: Vec<_> = by_plugin.keys().collect();
        plugins.sort();

        for plugin in plugins {
            let files_for_plugin = &by_plugin[plugin];
            println!(
                "📁 {} ({} files)",
                plugin.to_uppercase(),
                files_for_plugin.len()
            );

            for file in files_for_plugin {
                let size_kb = file.size_bytes as f64 / 1024.0;
                println!("   {} ({:.1} KB)", file.relative_path.display(), size_kb);
            }
            println!();
        }

        if !unknown_files.is_empty() {
            println!("❓ UNKNOWN ({} files)", unknown_files.len());
            for file in unknown_files {
                let size_kb = file.size_bytes as f64 / 1024.0;
                println!("   {} ({:.1} KB)", file.relative_path.display(), size_kb);
            }
            println!();
        }

        let total_size_mb: f64 =
            files.iter().map(|f| f.size_bytes as f64).sum::<f64>() / (1024.0 * 1024.0);

        println!("📊 Summary:");
        println!("   Plugins detected: {}", by_plugin.len());
        println!(
            "   Text files: {}",
            files.iter().filter(|f| f.is_text).count()
        );
        println!("   Total size: {total_size_mb:.2} MB");

        let plugin_summary = self.config.get_plugin_summary();
        println!("\n🔌 Plugin Configuration:");
        println!(
            "   Input plugins: {} enabled / {} total",
            plugin_summary.enabled_input_plugins, plugin_summary.total_input_plugins
        );
        println!(
            "   Output plugins: {} enabled / {} total",
            plugin_summary.enabled_output_plugins, plugin_summary.total_output_plugins
        );
    }
}
