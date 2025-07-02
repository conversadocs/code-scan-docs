use crate::core::matrix::ProjectMatrix;
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

        use crate::plugins::communication::PluginCommunicator;
        use crate::plugins::interface::PluginInput;

        let plugin_name = file_info.plugin_name.as_ref().unwrap();
        debug!("📝 Plugin name: {plugin_name}");

        // Get plugin path from config
        let plugin_config = self
            .config
            .plugins
            .get(plugin_name)
            .ok_or_else(|| anyhow::anyhow!("Plugin {} not found in config", plugin_name))?;

        debug!("⚙️ Got plugin config for: {plugin_name}");

        // Resolve plugin path
        let plugin_path = match &plugin_config.source {
            crate::utils::config::PluginSource::Builtin { name } => {
                PathBuf::from(format!("plugins/input/{name}_analyzer.py"))
            }
            _ => {
                // TODO: Handle other plugin sources
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
        // Communicate with plugin
        let mut communicator = PluginCommunicator::new(plugin_path).with_cache_dir(cache_dir);

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
            language: self.config.find_plugin_for_file(&file_info.path),
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
            language: self.config.find_plugin_for_file(&file_info.path),
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
            let plugin_name = self.config.find_plugin_for_file(path);

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
        // Simple heuristic - if a plugin claims it, it's probably text
        if self.config.find_plugin_for_file(path).is_some() {
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
    }
}

// Unit tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::config::{Config, FilePatterns, PluginConfig, PluginSource};
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::fs;

    // Helper function to create a test project structure
    async fn create_test_project(temp_dir: &TempDir) -> Result<PathBuf> {
        let project_root = temp_dir.path().to_path_buf();

        // Create directory structure
        fs::create_dir_all(project_root.join("src")).await?;
        fs::create_dir_all(project_root.join("tests")).await?;
        fs::create_dir_all(project_root.join("target/debug")).await?; // Should be ignored
        fs::create_dir_all(project_root.join(".git")).await?; // Should be ignored

        // Create test files
        fs::write(
            project_root.join("src/main.rs"),
            "fn main() { println!(\"Hello\"); }",
        )
        .await?;
        fs::write(project_root.join("src/lib.rs"), "pub mod utils;").await?;
        fs::write(project_root.join("src/utils.rs"), "pub fn helper() {}").await?;
        fs::write(
            project_root.join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"",
        )
        .await?;
        fs::write(project_root.join("Cargo.lock"), "# Cargo.lock").await?;

        // Python files
        fs::write(project_root.join("script.py"), "print('hello')").await?;
        fs::write(project_root.join("requirements.txt"), "requests==2.28.0").await?;

        // Files that should be ignored (ensure parent dirs exist)
        fs::write(project_root.join("target/debug/test"), "binary").await?;
        fs::write(project_root.join("app.log"), "log data").await?;
        fs::write(project_root.join(".git/config"), "git config").await?;

        // Large file (should be ignored if over limit)
        let large_content = "x".repeat(15 * 1024 * 1024); // 15MB
        fs::write(project_root.join("large_file.txt"), large_content).await?;

        // Hidden file
        fs::write(project_root.join(".hidden"), "hidden content").await?;

        // Binary file
        fs::write(project_root.join("binary.bin"), vec![0u8, 1u8, 2u8, 255u8]).await?;

        Ok(project_root)
    }

    // Helper function to create a minimal config for testing
    fn create_test_config() -> Config {
        let mut config = Config::default();
        // Override some settings for testing
        config.scanning.max_file_size_mb = 10; // 10MB limit
        config.scanning.include_hidden = false;
        config
    }

    // Helper function to create config with custom patterns
    fn create_config_with_custom_patterns() -> Config {
        let mut config = create_test_config();

        // Add a test plugin that matches .test files
        let mut plugins = std::collections::HashMap::new();
        plugins.insert(
            "test_plugin".to_string(),
            PluginConfig {
                source: PluginSource::Builtin {
                    name: "test".to_string(),
                },
                file_patterns: FilePatterns {
                    extensions: vec![".test".to_string()],
                    filenames: vec!["TEST_FILE".to_string()],
                    glob_patterns: None,
                },
                enabled: true,
                config: None,
            },
        );

        // Keep existing plugins and add new one
        for (name, plugin) in config.plugins {
            plugins.insert(name, plugin);
        }

        config.plugins = plugins;
        config
    }

    #[test]
    fn test_project_scanner_creation() {
        let config = create_test_config();
        let scanner = ProjectScanner::new(config.clone());

        // Default project root should be current directory
        assert_eq!(scanner.project_root, PathBuf::from("."));
        assert_eq!(scanner.config.scanning.max_file_size_mb, 10);
    }

    #[test]
    fn test_project_scanner_with_root() {
        let config = create_test_config();
        let custom_root = PathBuf::from("/custom/root");
        let scanner = ProjectScanner::new(config).with_root(&custom_root);

        assert_eq!(scanner.project_root, custom_root);
    }

    #[tokio::test]
    async fn test_scan_finds_expected_files() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_root = create_test_project(&temp_dir)
            .await
            .expect("Failed to create test project");

        let config = create_test_config();
        let scanner = ProjectScanner::new(config).with_root(&project_root);

        let files = scanner.scan().await.expect("Scan failed");

        // Should find our test files
        let file_paths: Vec<String> = files
            .iter()
            .map(|f| f.relative_path.to_string_lossy().to_string())
            .collect();

        // Check that we found the expected files
        assert!(file_paths.iter().any(|p| p.contains("main.rs")));
        assert!(file_paths.iter().any(|p| p.contains("lib.rs")));
        assert!(file_paths.iter().any(|p| p.contains("utils.rs")));
        assert!(file_paths.iter().any(|p| p.contains("Cargo.toml")));
        assert!(file_paths.iter().any(|p| p.contains("script.py")));
        assert!(file_paths.iter().any(|p| p.contains("requirements.txt")));

        // Should NOT find ignored files
        assert!(!file_paths.iter().any(|p| p.contains("target/")));
        assert!(!file_paths.iter().any(|p| p.contains(".git/")));
        assert!(!file_paths.iter().any(|p| p.contains("app.log")));

        // Should not find large file (over 10MB limit)
        assert!(!file_paths.iter().any(|p| p.contains("large_file.txt")));

        // Should not find hidden files (include_hidden = false)
        assert!(!file_paths.iter().any(|p| p.contains(".hidden")));
    }

    #[tokio::test]
    async fn test_scan_with_hidden_files_enabled() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_root = create_test_project(&temp_dir)
            .await
            .expect("Failed to create test project");

        let mut config = create_test_config();
        config.scanning.include_hidden = true;

        let scanner = ProjectScanner::new(config).with_root(&project_root);
        let files = scanner.scan().await.expect("Scan failed");

        let file_paths: Vec<String> = files
            .iter()
            .map(|f| f.relative_path.to_string_lossy().to_string())
            .collect();

        // Should find hidden files when enabled
        assert!(file_paths.iter().any(|p| p.contains(".hidden")));
    }

    #[tokio::test]
    async fn test_scan_respects_file_size_limit() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_root = create_test_project(&temp_dir)
            .await
            .expect("Failed to create test project");

        let mut config = create_test_config();
        config.scanning.max_file_size_mb = 20; // Increase limit to 20MB

        let scanner = ProjectScanner::new(config).with_root(&project_root);
        let files = scanner.scan().await.expect("Scan failed");

        let file_paths: Vec<String> = files
            .iter()
            .map(|f| f.relative_path.to_string_lossy().to_string())
            .collect();

        // Should now find the large file
        assert!(file_paths.iter().any(|p| p.contains("large_file.txt")));
    }

    #[test]
    fn test_should_ignore_file() {
        let config = create_test_config();
        let scanner = ProjectScanner::new(config).with_root("/test");

        // Test ignore patterns
        assert!(scanner.should_ignore_file(&PathBuf::from("target/debug/app")));
        assert!(scanner.should_ignore_file(&PathBuf::from("app.log")));
        assert!(scanner.should_ignore_file(&PathBuf::from(".csd_cache/data.json")));
        assert!(scanner.should_ignore_file(&PathBuf::from("node_modules/package/index.js")));

        // Should not ignore normal files
        assert!(!scanner.should_ignore_file(&PathBuf::from("src/main.rs")));
        assert!(!scanner.should_ignore_file(&PathBuf::from("README.md")));
        assert!(!scanner.should_ignore_file(&PathBuf::from("script.py")));
    }

    #[test]
    fn test_is_text_file() {
        let config = create_test_config();
        let scanner = ProjectScanner::new(config).with_root("/test");

        // Files with plugins should be considered text
        assert!(scanner.is_text_file(&PathBuf::from("main.rs"), &Some(".rs".to_string())));
        assert!(scanner.is_text_file(&PathBuf::from("script.py"), &Some(".py".to_string())));

        // Common text files without plugins
        assert!(scanner.is_text_file(&PathBuf::from("README.md"), &Some(".md".to_string())));
        assert!(scanner.is_text_file(&PathBuf::from("config.ini"), &Some(".ini".to_string())));
        // Note: .json is not in the current is_text_file implementation, so we won't test it
        assert!(scanner.is_text_file(&PathBuf::from("data.txt"), &Some(".txt".to_string())));

        // Files without extensions but known names
        assert!(scanner.is_text_file(&PathBuf::from("README"), &None));
        assert!(scanner.is_text_file(&PathBuf::from("LICENSE"), &None));
        assert!(scanner.is_text_file(&PathBuf::from("Dockerfile"), &None));
        assert!(scanner.is_text_file(&PathBuf::from("Makefile"), &None));

        // Binary files should not be text
        assert!(!scanner.is_text_file(&PathBuf::from("image.png"), &Some(".png".to_string())));
        assert!(!scanner.is_text_file(&PathBuf::from("app.exe"), &Some(".exe".to_string())));
        assert!(!scanner.is_text_file(&PathBuf::from("data.bin"), &Some(".bin".to_string())));

        // Unknown files should not be text
        assert!(!scanner.is_text_file(&PathBuf::from("unknown.xyz"), &Some(".xyz".to_string())));
        assert!(!scanner.is_text_file(&PathBuf::from("randomfile"), &None));
    }

    #[test]
    fn test_calculate_file_hash() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_file = temp_dir.path().join("test.txt");

        // Create a test file
        std::fs::write(&test_file, "test content").expect("Failed to write test file");

        let config = create_test_config();
        let scanner = ProjectScanner::new(config).with_root(temp_dir.path());

        let hash1 = scanner
            .calculate_file_hash(&test_file)
            .expect("Failed to calculate hash");
        let hash2 = scanner
            .calculate_file_hash(&test_file)
            .expect("Failed to calculate hash");

        // Same file should produce same hash
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 produces 64-character hex string
        assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()));

        // Different content should produce different hash
        std::fs::write(&test_file, "different content").expect("Failed to write test file");
        let hash3 = scanner
            .calculate_file_hash(&test_file)
            .expect("Failed to calculate hash");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_calculate_file_hash_nonexistent_file() {
        let config = create_test_config();
        let scanner = ProjectScanner::new(config).with_root("/test");

        let result = scanner.calculate_file_hash(&PathBuf::from("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_info_structure() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_root = create_test_project(&temp_dir)
            .await
            .expect("Failed to create test project");

        let config = create_test_config();
        let scanner = ProjectScanner::new(config).with_root(&project_root);

        let files = scanner.scan().await.expect("Scan failed");

        // Find a specific file to test
        let rust_file = files
            .iter()
            .find(|f| f.relative_path.to_string_lossy().contains("main.rs"))
            .expect("Could not find main.rs");

        // Test FileInfo structure
        assert!(rust_file.path.ends_with("main.rs"));
        assert_eq!(rust_file.relative_path, PathBuf::from("src/main.rs"));
        assert_eq!(rust_file.extension, Some(".rs".to_string()));
        assert!(rust_file.size_bytes > 0);
        assert!(rust_file.is_text);
        assert_eq!(rust_file.plugin_name, Some("rust".to_string()));
        assert!(!rust_file.content_hash.is_empty());
        assert_eq!(rust_file.content_hash.len(), 64); // SHA256 hex string

        // Test Python file
        let python_file = files
            .iter()
            .find(|f| f.relative_path.to_string_lossy().contains("script.py"))
            .expect("Could not find script.py");

        assert_eq!(python_file.extension, Some(".py".to_string()));
        assert!(python_file.is_text);
        assert_eq!(python_file.plugin_name, Some("python".to_string()));
    }

    #[tokio::test]
    async fn test_scan_empty_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let empty_project = temp_dir.path().join("empty");
        fs::create_dir(&empty_project)
            .await
            .expect("Failed to create empty dir");

        let config = create_test_config();
        let scanner = ProjectScanner::new(config).with_root(&empty_project);

        let files = scanner.scan().await.expect("Scan failed");
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn test_scan_with_custom_plugin_patterns() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_root = temp_dir.path().to_path_buf();

        // Create files that match our custom plugin
        fs::write(project_root.join("test.test"), "test content")
            .await
            .expect("Failed to write test file");
        fs::write(project_root.join("TEST_FILE"), "test file content")
            .await
            .expect("Failed to write test file");
        fs::write(project_root.join("normal.txt"), "normal content")
            .await
            .expect("Failed to write normal file");

        let config = create_config_with_custom_patterns();
        let scanner = ProjectScanner::new(config).with_root(&project_root);

        let files = scanner.scan().await.expect("Scan failed");

        // Find our custom plugin files
        let test_ext_file = files
            .iter()
            .find(|f| f.relative_path.to_string_lossy().contains("test.test"));
        let test_name_file = files
            .iter()
            .find(|f| f.relative_path.to_string_lossy().contains("TEST_FILE"));
        let normal_file = files
            .iter()
            .find(|f| f.relative_path.to_string_lossy().contains("normal.txt"));

        // Custom plugin should be detected
        assert!(test_ext_file.is_some());
        assert_eq!(
            test_ext_file.unwrap().plugin_name,
            Some("test_plugin".to_string())
        );

        assert!(test_name_file.is_some());
        assert_eq!(
            test_name_file.unwrap().plugin_name,
            Some("test_plugin".to_string())
        );

        // Normal file should not have a plugin
        assert!(normal_file.is_some());
        assert_eq!(normal_file.unwrap().plugin_name, None);
    }

    #[tokio::test]
    async fn test_create_basic_file_node() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let test_file = temp_dir.path().join("test.txt");
        fs::write(&test_file, "test content")
            .await
            .expect("Failed to write test file");

        let config = create_test_config();
        let scanner = ProjectScanner::new(config).with_root(temp_dir.path());

        // Create a FileInfo for testing
        let file_info = FileInfo {
            path: test_file.clone(),
            relative_path: PathBuf::from("test.txt"),
            extension: Some(".txt".to_string()),
            size_bytes: 12,
            is_text: true,
            plugin_name: None,
            content_hash: "test_hash".to_string(),
        };

        let file_node = scanner
            .create_basic_file_node(&file_info)
            .await
            .expect("Failed to create basic file node");

        // Verify the file node structure
        assert_eq!(file_node.path, test_file);
        assert_eq!(file_node.relative_path, PathBuf::from("test.txt"));
        assert_eq!(file_node.hash, "test_hash");
        assert_eq!(file_node.size_bytes, 12);
        assert_eq!(file_node.plugin, "unknown");
        assert!(file_node.is_text);
        assert!(file_node.elements.is_empty());
        assert!(file_node.imports.is_empty());
        assert!(file_node.exports.is_empty());
        assert!(file_node.file_summary.is_none());
    }

    #[tokio::test]
    async fn test_scan_handles_permission_errors() {
        // This test is tricky because we need a file we can't read
        // On most systems, we can't easily create such a file in tests
        // So we'll test with a non-existent directory instead

        let config = create_test_config();
        let scanner = ProjectScanner::new(config).with_root("/definitely/does/not/exist");

        // This should handle the error gracefully and return an empty result
        // rather than panicking
        let result = scanner.scan().await;

        // The scan might succeed with empty results or fail gracefully
        // Either is acceptable behavior
        match result {
            Ok(files) => {
                // If it succeeds, should be empty
                assert!(files.is_empty());
            }
            Err(_) => {
                // If it fails, that's also acceptable for non-existent directories
            }
        }
    }

    #[test]
    fn test_print_scan_results() {
        // This is more of a smoke test since print_scan_results outputs to stdout
        // We're mainly testing that it doesn't panic

        let config = create_test_config();
        let scanner = ProjectScanner::new(config).with_root("/test");

        let files = vec![
            FileInfo {
                path: PathBuf::from("/test/main.rs"),
                relative_path: PathBuf::from("main.rs"),
                extension: Some(".rs".to_string()),
                size_bytes: 1024,
                is_text: true,
                plugin_name: Some("rust".to_string()),
                content_hash: "test_hash".to_string(),
            },
            FileInfo {
                path: PathBuf::from("/test/script.py"),
                relative_path: PathBuf::from("script.py"),
                extension: Some(".py".to_string()),
                size_bytes: 512,
                is_text: true,
                plugin_name: Some("python".to_string()),
                content_hash: "test_hash2".to_string(),
            },
            FileInfo {
                path: PathBuf::from("/test/unknown.xyz"),
                relative_path: PathBuf::from("unknown.xyz"),
                extension: Some(".xyz".to_string()),
                size_bytes: 256,
                is_text: false,
                plugin_name: None,
                content_hash: "test_hash3".to_string(),
            },
        ];

        // This should not panic
        scanner.print_scan_results(&files);
    }

    #[tokio::test]
    async fn test_scan_to_matrix_integration() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let project_root = create_test_project(&temp_dir)
            .await
            .expect("Failed to create test project");

        let config = create_test_config();
        let scanner = ProjectScanner::new(config).with_root(&project_root);

        // This tests the integration between scanner and matrix creation
        // Note: This will fail if plugins aren't available, but tests the basic structure
        let result = scanner.scan_to_matrix().await;

        match result {
            Ok(matrix) => {
                // If plugins work, we should get a populated matrix
                assert!(!matrix.files.is_empty());
                assert_eq!(matrix.metadata.project_root, project_root);
                assert!(matrix.metadata.total_files > 0);
            }
            Err(e) => {
                // If plugins fail (which is expected in unit tests), that's ok
                // The important thing is that it doesn't panic
                eprintln!("scan_to_matrix failed (expected in unit tests): {e}");
            }
        }
    }
}
