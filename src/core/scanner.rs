use anyhow::Result;
use log::{info, debug, warn};
use sha2::{Sha256, Digest};
use std::path::{Path, PathBuf};
use ignore::WalkBuilder;
use crate::utils::config::Config;
use crate::core::matrix::{ProjectMatrix, FileNode};

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
        info!("Starting file scan and matrix creation in: {}", self.project_root.display());

        let mut matrix = ProjectMatrix::new(self.project_root.clone());
        let files = self.scan().await?;

        info!("Found {} files, building matrix...", files.len());

        // Convert files to matrix nodes
        for file_info in files {
            // For now, create basic file nodes without plugin analysis
            // TODO: Later we'll call plugins to populate elements, imports, etc.

            let file_node = FileNode {
                path: file_info.path.clone(),
                relative_path: file_info.relative_path,
                hash: file_info.content_hash,
                size_bytes: file_info.size_bytes,
                plugin: file_info.plugin_name.unwrap_or_else(|| "unknown".to_string()),
                language: self.config.find_plugin_for_file(&file_info.path),
                is_text: file_info.is_text,
                elements: Vec::new(),     // TODO: Populate from plugin
                imports: Vec::new(),      // TODO: Populate from plugin
                exports: Vec::new(),      // TODO: Populate from plugin
                file_summary: None,      // TODO: Generate with LLM
            };

            matrix.add_file(file_node);
        }

        info!("Matrix created with {} files", matrix.files.len());
        Ok(matrix)
    }

    pub async fn scan(&self) -> Result<Vec<FileInfo>> {
        info!("Starting file scan in: {}", self.project_root.display());

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
                    warn!("Error reading directory entry: {}", e);
                    continue;
                }
            };

            _total_files += 1;

            // Skip directories
            if entry.file_type().map_or(false, |ft| ft.is_dir()) {
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
                debug!("File too large, skipping: {} ({} bytes)", path.display(), size_bytes);
                skipped_files += 1;
                continue;
            }

            // Create relative path
            let relative_path = match path.strip_prefix(&self.project_root) {
                Ok(rel) => rel.to_path_buf(),
                Err(_) => path.to_path_buf(),
            };

            // Detect file info
            let extension = path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| format!(".{}", ext.to_lowercase()));

            let is_text = self.is_text_file(path, &extension);
            let plugin_name = self.config.find_plugin_for_file(path);

            // Calculate content hash
            let content_hash = self.calculate_file_hash(path).unwrap_or_else(|_| "error".to_string());

            let file_info = FileInfo {
                path: path.to_path_buf(),
                relative_path,
                extension,
                size_bytes,
                is_text,
                plugin_name,
                content_hash,
            };

            debug!("Found file: {:?}", file_info);
            files.push(file_info);
        }

        info!("Scan complete. Found {} files, skipped {} files", files.len(), skipped_files);

        Ok(files)
    }

    fn calculate_file_hash(&self, path: &Path) -> Result<String> {
        let content = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    fn should_ignore_file(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        for pattern in &self.config.scanning.ignore_patterns {
            // Simple glob-like matching
            if pattern.ends_with('/') {
                // Directory pattern
                let dir_pattern = &pattern[..pattern.len()-1];
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
                ".md" | ".rst" | ".txt" | ".asciidoc" | ".adoc" | ".org" | ".tex" |
                ".ini" | ".cfg" | ".conf" | ".properties" | ".env" |
                ".gitignore" | ".gitattributes" | ".dockerignore" | ".editorconfig" => true,
                _ => false,
            }
        } else {
            // Check files without extensions by name
            let filename = path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            matches!(filename.to_lowercase().as_str(),
                "readme" | "license" | "copyright" | "authors" | "contributors" |
                "changelog" | "news" | "dockerfile" | "makefile" |
                ".gitignore" | ".gitattributes" | ".dockerignore" | ".editorconfig"
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
            println!("📁 {} ({} files)", plugin.to_uppercase(), files_for_plugin.len());

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
        let total_size_mb: f64 = files.iter()
            .map(|f| f.size_bytes as f64)
            .sum::<f64>() / (1024.0 * 1024.0);

        println!("📊 Summary:");
        println!("   Plugins detected: {}", by_plugin.len());
        println!("   Text files: {}", files.iter().filter(|f| f.is_text).count());
        println!("   Total size: {:.2} MB", total_size_mb);
    }
}
