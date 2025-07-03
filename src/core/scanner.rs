use crate::core::matrix::ProjectMatrix;
use crate::plugins::interface::{InputPluginInterface, PluginInput};
use crate::utils::config::Config;
use anyhow::Result;
use ignore::WalkBuilder;
use log::{debug, info, warn};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

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

pub struct ProjectScanner {
    config: Config,
    project_root: PathBuf,
}

impl ProjectScanner {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            project_root: PathBuf::from("."),
        }
    }

    pub fn with_root<P: AsRef<Path>>(mut self, root: P) -> Self {
        self.project_root = root.as_ref().to_path_buf();
        self
    }

    pub async fn scan_to_matrix(&self) -> Result<ProjectMatrix> {
        debug!(
            "Starting file scan and matrix creation in: {}",
            self.project_root.display()
        );

        let mut matrix = ProjectMatrix::new(self.project_root.clone());
        let files = self.scan().await?;

        debug!("Found {} files, analyzing with plugins...", files.len());

        // Convert files to matrix nodes with plugin analysis
        for file_info in files {
            debug!(
                "🔍 Processing file: {} (is_text: {}, plugin: {:?})",
                file_info.path.display(),
                file_info.is_text,
                file_info.plugin_name
            );

            let file_node = if file_info.is_text && file_info.plugin_name.is_some() {
                debug!("✅ Calling plugin for: {}", file_info.path.display());
                // Analyze with plugin
                self.analyze_file_with_plugin(&file_info, &mut matrix)
                    .await?
            } else {
                debug!(
                    "❌ Skipping plugin for: {} (is_text: {}, plugin: {:?})",
                    file_info.path.display(),
                    file_info.is_text,
                    file_info.plugin_name
                );
                // Create basic file node without plugin analysis
                self.create_basic_file_node(&file_info).await?
            };

            matrix.add_file(file_node);
        }

        debug!("Matrix created with {} files", matrix.files.len());
        Ok(matrix)
    }

    async fn analyze_file_with_plugin(
        &self,
        file_info: &FileInfo,
        matrix: &mut ProjectMatrix,
    ) -> Result<crate::core::matrix::FileNode> {
        info!("🚀 Starting analysis for: {}", file_info.path.display());

        use crate::plugins::communication::InputPluginCommunicator;

        let plugin_name = file_info.plugin_name.as_ref().unwrap();
        debug!("📝 Plugin name: {plugin_name}");

        // Get input plugin configuration from new structure
        let plugin_config = self
            .config
            .get_input_plugin(plugin_name)
            .ok_or_else(|| anyhow::anyhow!("Input plugin {} not found in config", plugin_name))?;

        debug!("⚙️ Got input plugin config for: {plugin_name}");

        // Resolve plugin path
        let plugin_path = match &plugin_config.source {
            crate::utils::config::PluginSource::Builtin { name } => {
                PathBuf::from(format!("plugins/input/{name}_analyzer.py"))
            }
            crate::utils::config::PluginSource::Local { path } => PathBuf::from(path),
            _ => {
                // TODO: Handle other plugin sources (GitHub, Git)
                return self.create_basic_file_node(file_info).await;
            }
        };

        debug!("📂 Plugin path resolved to: {}", plugin_path.display());

        // Check if plugin file exists
        if !plugin_path.exists() {
            warn!("Plugin file not found: {}", plugin_path.display());
            return self.create_basic_file_node(file_info).await;
        }

        debug!("✅ Plugin file exists");

        // Read file content
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

        // Set up cache directory
        let cache_dir = self.project_root.join(".csd_cache");

        debug!("🔧 Creating plugin input...");
        // Create plugin input
        let plugin_input = PluginInput {
            file_path: file_info.path.clone(),
            relative_path: file_info.relative_path.clone(),
            content,
            project_root: self.project_root.clone(),
            cache_dir: cache_dir.to_string_lossy().to_string(),
            plugin_config: plugin_config.config.as_ref().map(|v| {
                // Convert serde_yaml::Value to serde_json::Value
                serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
            }),
        };

        debug!("📡 Creating plugin communicator...");
        // Communicate with plugin using the new InputPluginCommunicator
        let mut communicator = InputPluginCommunicator::new(plugin_path).with_cache_dir(cache_dir);

        // Use configured Python executable or auto-detect
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

                // Convert plugin output to matrix data
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

    async fn convert_plugin_output_to_file_node(
        &self,
        file_info: &FileInfo,
        plugin_output: crate::plugins::interface::PluginOutput,
        matrix: &mut ProjectMatrix,
    ) -> Result<crate::core::matrix::FileNode> {
        use crate::core::matrix::{ExternalDependency, Relationship};

        // Convert plugin CodeElements to matrix CodeElements
        let elements: Vec<crate::core::matrix::CodeElement> = plugin_output
            .elements
            .into_iter()
            .map(|e| {
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
                        _ => crate::core::matrix::ElementType::Function, // Default fallback
                    },
                    name: e.name,
                    signature: e.signature,
                    line_start: e.line_start,
                    line_end: e.line_end,
                    summary: e.summary,
                    complexity_score: e.complexity_score,
                    calls: e.calls,
                    metadata: e.metadata,
                }
            })
            .collect();

        // Convert plugin Imports to matrix Imports
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

        // Add relationships to the matrix
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

        // Add external dependencies to the matrix
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

        // Create the file node
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
        })
    }

    async fn create_basic_file_node(
        &self,
        file_info: &FileInfo,
    ) -> Result<crate::core::matrix::FileNode> {
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
        })
    }

    pub async fn scan(&self) -> Result<Vec<FileInfo>> {
        debug!("Starting file scan in: {}", self.project_root.display());

        let mut files = Vec::new();
        let mut _total_files = 0;
        let mut skipped_files = 0;

        // Use the `ignore` crate to respect .gitignore, .ignore files
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

            // Skip directories
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                continue;
            }

            let path = entry.path();

            // Check if file matches our ignore patterns
            if self.should_ignore_file(path) {
                debug!("Ignoring file: {}", path.display());
                skipped_files += 1;
                continue;
            }

            // Check file size
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

            // Create relative path
            let relative_path = match path.strip_prefix(&self.project_root) {
                Ok(rel) => rel.to_path_buf(),
                Err(_) => path.to_path_buf(),
            };

            // Detect file info
            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| format!(".{}", ext.to_lowercase()));

            let is_text = self.is_text_file(path, &extension);
            let plugin_name = self.config.find_input_plugin_for_file(path);

            // Calculate content hash
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

    fn calculate_file_hash(&self, path: &Path) -> Result<String> {
        let content = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        Ok(format!("{hash:x}"))
    }

    fn should_ignore_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.config.scanning.ignore_patterns {
            // Simple glob-like matching
            if pattern.ends_with('/') {
                // Directory pattern
                let dir_pattern = &pattern[..pattern.len() - 1];
                if path_str.contains(dir_pattern) {
                    return true;
                }
            } else if pattern.starts_with("*.") {
                // Extension pattern
                let ext = &pattern[1..]; // Remove the *
                if path_str.ends_with(ext) {
                    return true;
                }
            } else if path_str.contains(pattern) {
                // Simple substring match
                return true;
            }
        }

        false
    }

    fn is_text_file(&self, path: &Path, extension: &Option<String>) -> bool {
        // Simple heuristic - if an input plugin claims it, it's probably text
        if self.config.find_input_plugin_for_file(path).is_some() {
            return true;
        }

        // Check by extension for common text files not handled by plugins
        if let Some(ext) = extension {
            match ext.as_str() {
                // Documentation and config
                ".md" | ".rst" | ".txt" | ".asciidoc" | ".adoc" | ".org" | ".tex" | ".ini"
                | ".cfg" | ".conf" | ".properties" | ".env" | ".gitignore" | ".gitattributes"
                | ".dockerignore" | ".editorconfig" => true,
                _ => false,
            }
        } else {
            // Check files without extensions by name
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

    pub fn print_scan_results(&self, files: &[FileInfo]) {
        println!("\n=== CSD File Scan Results ===");
        println!("Project root: {}", self.project_root.display());
        println!("Total files found: {}\n", files.len());

        // Group by plugin
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

        // Print by plugin
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

        // Print unknown files
        if !unknown_files.is_empty() {
            println!("❓ UNKNOWN ({} files)", unknown_files.len());
            for file in unknown_files {
                let size_kb = file.size_bytes as f64 / 1024.0;
                println!("   {} ({:.1} KB)", file.relative_path.display(), size_kb);
            }
            println!();
        }

        // Summary
        let total_size_mb: f64 =
            files.iter().map(|f| f.size_bytes as f64).sum::<f64>() / (1024.0 * 1024.0);

        println!("📊 Summary:");
        println!("   Plugins detected: {}", by_plugin.len());
        println!(
            "   Text files: {}",
            files.iter().filter(|f| f.is_text).count()
        );
        println!("   Total size: {total_size_mb:.2} MB");

        // Show plugin configuration summary
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
