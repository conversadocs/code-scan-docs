use anyhow::Result;
use log::{info, debug, warn};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use ignore::WalkBuilder;
use crate::utils::config::Config;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub relative_path: PathBuf,
    pub extension: Option<String>,
    pub size_bytes: u64,
    pub is_text: bool,
    pub language: Option<String>,
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

    pub async fn scan(&self) -> Result<Vec<FileInfo>> {
        info!("Starting file scan in: {}", self.project_root.display());

        let mut files = Vec::new();
        let mut total_files = 0;
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

            total_files += 1;

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

            let file_info = FileInfo {
                path: path.to_path_buf(),
                relative_path,
                extension,
                size_bytes,
                is_text,
                language: plugin_name,
            };

            debug!("Found file: {:?}", file_info);
            files.push(file_info);
        }

        info!("Scan complete. Found {} files, skipped {} files", files.len(), skipped_files);

        Ok(files)
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

    fn detect_language(&self, path: &Path, extension: &Option<String>) -> Option<String> {
        // First check by filename (ecosystem files)
        let filename = path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        if let Some(lang) = self.detect_language_by_filename(filename) {
            return Some(lang);
        }

        // Then check by extension
        extension.as_ref().and_then(|ext| {
            match ext.as_str() {
                ".rs" => Some("rust".to_string()),
                ".py" => Some("python".to_string()),
                ".js" | ".mjs" => Some("javascript".to_string()),
                ".ts" => Some("typescript".to_string()),
                ".jsx" => Some("jsx".to_string()),
                ".tsx" => Some("tsx".to_string()),
                ".java" => Some("java".to_string()),
                ".c" => Some("c".to_string()),
                ".cpp" | ".cxx" | ".cc" => Some("cpp".to_string()),
                ".h" => Some("c_header".to_string()),
                ".hpp" | ".hxx" => Some("cpp_header".to_string()),
                ".cs" => Some("csharp".to_string()),
                ".go" => Some("go".to_string()),
                ".rb" => Some("ruby".to_string()),
                ".php" => Some("php".to_string()),
                ".swift" => Some("swift".to_string()),
                ".kt" => Some("kotlin".to_string()),
                ".scala" => Some("scala".to_string()),
                ".html" | ".htm" => Some("html".to_string()),
                ".css" => Some("css".to_string()),
                ".scss" => Some("scss".to_string()),
                ".sass" => Some("sass".to_string()),
                ".less" => Some("less".to_string()),
                ".json" => Some("json".to_string()),
                ".yaml" | ".yml" => Some("yaml".to_string()),
                ".xml" => Some("xml".to_string()),
                ".md" => Some("markdown".to_string()),
                ".sh" | ".bash" => Some("bash".to_string()),
                ".ps1" => Some("powershell".to_string()),
                ".sql" => Some("sql".to_string()),

                // Generic config files - only if not caught by filename detection above
                ".toml" => Some("toml".to_string()),
                ".ini" | ".cfg" | ".conf" => Some("config".to_string()),

                _ => None,
            }
        })
    }

    fn detect_language_by_filename(&self, filename: &str) -> Option<String> {
        match filename.to_lowercase().as_str() {
            // Rust ecosystem
            "cargo.toml" | "cargo.lock" => Some("rust".to_string()),
            ".rustfmt.toml" | "rust-toolchain.toml" => Some("rust".to_string()),

            // Python ecosystem
            "requirements.txt" | "requirements-dev.txt" | "requirements-test.txt" => Some("python".to_string()),
            "pyproject.toml" | "setup.py" | "setup.cfg" => Some("python".to_string()),
            "pipfile" | "pipfile.lock" | "poetry.lock" => Some("python".to_string()),
            "conda.yaml" | "environment.yml" => Some("python".to_string()),
            "tox.ini" | "pytest.ini" | ".flake8" | ".pylintrc" => Some("python".to_string()),

            // JavaScript/Node.js ecosystem
            "package.json" | "package-lock.json" | "yarn.lock" => Some("javascript".to_string()),
            "tsconfig.json" | "jsconfig.json" => Some("typescript".to_string()),
            "webpack.config.js" | "vite.config.js" | "rollup.config.js" => Some("javascript".to_string()),
            ".eslintrc.json" | ".eslintrc.js" | ".prettierrc" => Some("javascript".to_string()),
            "babel.config.js" | ".babelrc" => Some("javascript".to_string()),

            // Java ecosystem
            "pom.xml" | "build.gradle" | "build.gradle.kts" => Some("java".to_string()),
            "gradle.properties" | "settings.gradle" => Some("java".to_string()),

            // Go ecosystem
            "go.mod" | "go.sum" => Some("go".to_string()),

            // Ruby ecosystem
            "gemfile" | "gemfile.lock" | "rakefile" => Some("ruby".to_string()),
            ".ruby-version" | ".ruby-gemset" => Some("ruby".to_string()),

            // PHP ecosystem
            "composer.json" | "composer.lock" => Some("php".to_string()),

            // .NET ecosystem
            "global.json" | "nuget.config" => Some("csharp".to_string()),

            // C/C++ ecosystem
            "makefile" | "cmake" | "cmakelists.txt" => Some("cpp".to_string()),
            "configure.ac" | "configure.in" => Some("cpp".to_string()),

            // Docker ecosystem
            "dockerfile" | "docker-compose.yml" | "docker-compose.yaml" => Some("docker".to_string()),
            ".dockerignore" => Some("docker".to_string()),

            // CI/CD files
            ".gitlab-ci.yml" => Some("gitlab_ci".to_string()),
            "jenkinsfile" => Some("jenkins".to_string()),

            // General project files
            "readme.md" | "readme.txt" | "readme" => Some("documentation".to_string()),
            "license" | "copyright" | "authors" | "contributors" => Some("documentation".to_string()),
            "changelog.md" | "changelog" | "news.md" => Some("documentation".to_string()),

            // Git files
            ".gitignore" | ".gitattributes" | ".gitmodules" => Some("git".to_string()),

            // Editor config
            ".editorconfig" => Some("config".to_string()),

            _ => None,
        }
    }

    // Remove the unused helper method
    // fn get_filename_from_current_file(&self) -> String {

    pub fn print_scan_results(&self, files: &[FileInfo]) {
        println!("\n=== CSD File Scan Results ===");
        println!("Project root: {}", self.project_root.display());
        println!("Total files found: {}\n", files.len());

        // Group by language
        let mut by_language: std::collections::HashMap<String, Vec<&FileInfo>> =
            std::collections::HashMap::new();
        let mut unknown_files = Vec::new();

        for file in files {
            match &file.language {
                Some(lang) => {
                    by_language.entry(lang.clone()).or_default().push(file);
                }
                None => {
                    unknown_files.push(file);
                }
            }
        }

        // Print by language
        let mut languages: Vec<_> = by_language.keys().collect();
        languages.sort();

        for lang in languages {
            let files_for_lang = &by_language[lang];
            println!("📁 {} ({} files)", lang.to_uppercase(), files_for_lang.len());

            for file in files_for_lang {
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
        println!("   Languages detected: {}", by_language.len());
        println!("   Text files: {}", files.iter().filter(|f| f.is_text).count());
        println!("   Total size: {:.2} MB", total_size_mb);
    }
}
