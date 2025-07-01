use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub output_dir: String,
    pub llm: LlmConfig,
    pub scanning: ScanConfig,
    pub plugins: HashMap<String, PluginConfig>,
    pub python_executable: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub source: PluginSource,
    pub file_patterns: FilePatterns,
    pub enabled: bool,
    pub config: Option<serde_yaml::Value>, // Plugin-specific configuration
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePatterns {
    pub extensions: Vec<String>,
    pub filenames: Vec<String>,
    pub glob_patterns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PluginSource {
    #[serde(rename = "local")]
    Local { path: String },
    #[serde(rename = "github")]
    GitHub {
        repo: String,
        version: Option<String>,
    },
    #[serde(rename = "git")]
    Git { url: String, branch: Option<String> },
    #[serde(rename = "builtin")]
    Builtin { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub ignore_patterns: Vec<String>,
    pub include_hidden: bool,
    pub max_file_size_mb: u64,
}

impl Default for Config {
    fn default() -> Self {
        let mut plugins = HashMap::new();

        // Built-in Python plugin for self-analysis
        plugins.insert(
            "python".to_string(),
            PluginConfig {
                source: PluginSource::Builtin {
                    name: "python".to_string(),
                },
                file_patterns: FilePatterns {
                    extensions: vec![".py".to_string()],
                    filenames: vec![
                        "requirements.txt".to_string(),
                        "setup.py".to_string(),
                        "pyproject.toml".to_string(),
                        "Pipfile".to_string(),
                        "poetry.lock".to_string(),
                        "tox.ini".to_string(),
                        "pytest.ini".to_string(),
                    ],
                    glob_patterns: Some(vec![
                        "requirements*.txt".to_string(),
                        "**/setup.py".to_string(),
                    ]),
                },
                enabled: true,
                config: None,
            },
        );

        // Built-in Rust plugin for self-analysis
        plugins.insert(
            "rust".to_string(),
            PluginConfig {
                source: PluginSource::Builtin {
                    name: "rust".to_string(),
                },
                file_patterns: FilePatterns {
                    extensions: vec![".rs".to_string()],
                    filenames: vec![
                        "Cargo.toml".to_string(),
                        "Cargo.lock".to_string(),
                        ".rustfmt.toml".to_string(),
                        "rust-toolchain.toml".to_string(),
                    ],
                    glob_patterns: Some(vec![
                        "**/Cargo.toml".to_string(),
                        "rust-toolchain*".to_string(),
                    ]),
                },
                enabled: true,
                config: None,
            },
        );

        Self {
            output_dir: "output".to_string(),
            llm: LlmConfig {
                provider: "ollama".to_string(),
                base_url: "http://localhost:11434".to_string(),
                model: "deepseek-coder".to_string(),
                timeout_seconds: 30,
            },
            scanning: ScanConfig {
                ignore_patterns: vec![
                    "target/".to_string(),
                    "node_modules/".to_string(),
                    ".git/".to_string(),
                    "*.log".to_string(),
                    ".csd_cache/".to_string(), // Our own cache directory
                ],
                include_hidden: false,
                max_file_size_mb: 10,
            },
            plugins,
            python_executable: None,
        }
    }
}

impl Config {
    pub async fn load(path: &Path) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub async fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Find which plugin should handle a given file
    pub fn find_plugin_for_file(&self, file_path: &Path) -> Option<String> {
        let filename = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!(".{}", ext.to_lowercase()));

        for (plugin_name, plugin_config) in &self.plugins {
            if !plugin_config.enabled {
                continue;
            }

            // Check extensions
            if let Some(ref ext) = extension {
                if plugin_config.file_patterns.extensions.contains(ext) {
                    return Some(plugin_name.clone());
                }
            }

            // Check exact filenames
            if plugin_config
                .file_patterns
                .filenames
                .iter()
                .any(|pattern| pattern.to_lowercase() == filename.to_lowercase())
            {
                return Some(plugin_name.clone());
            }

            // TODO: Check glob patterns if needed
        }

        None
    }
}

// Unit tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::fs;

    // Helper function to create a test config with custom plugins
    fn create_test_config_with_plugins() -> Config {
        let mut config = Config::default();

        // Add a custom JavaScript plugin for testing
        config.plugins.insert(
            "javascript".to_string(),
            PluginConfig {
                source: PluginSource::Builtin {
                    name: "javascript".to_string(),
                },
                file_patterns: FilePatterns {
                    extensions: vec![".js".to_string(), ".jsx".to_string()],
                    filenames: vec!["package.json".to_string()],
                    glob_patterns: Some(vec!["**/*.js".to_string()]),
                },
                enabled: true,
                config: None,
            },
        );

        config
    }

    // Helper function to create a minimal valid YAML config
    fn create_minimal_yaml_config() -> String {
        r#"
output_dir: "test_output"
python_executable: "python3"
llm:
  provider: "ollama"
  base_url: "http://localhost:11434"
  model: "test-model"
  timeout_seconds: 60
scanning:
  ignore_patterns:
    - "*.log"
    - "target/"
  include_hidden: false
  max_file_size_mb: 5
plugins:
  python:
    source:
      type: "builtin"
      name: "python"
    file_patterns:
      extensions:
        - ".py"
      filenames:
        - "requirements.txt"
    enabled: true
"#
        .to_string()
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();

        // Test basic properties
        assert_eq!(config.output_dir, "output");
        assert_eq!(config.llm.provider, "ollama");
        assert_eq!(config.llm.model, "deepseek-coder");
        assert_eq!(config.llm.timeout_seconds, 30);
        assert_eq!(config.python_executable, None);

        // Test scanning defaults
        assert!(!config.scanning.include_hidden);
        assert_eq!(config.scanning.max_file_size_mb, 10);
        assert!(config
            .scanning
            .ignore_patterns
            .contains(&"target/".to_string()));
        assert!(config
            .scanning
            .ignore_patterns
            .contains(&".csd_cache/".to_string()));

        // Test default plugins
        assert!(config.plugins.contains_key("python"));
        assert!(config.plugins.contains_key("rust"));

        let python_plugin = &config.plugins["python"];
        assert!(python_plugin.enabled);
        assert!(python_plugin
            .file_patterns
            .extensions
            .contains(&".py".to_string()));
        assert!(python_plugin
            .file_patterns
            .filenames
            .contains(&"requirements.txt".to_string()));

        let rust_plugin = &config.plugins["rust"];
        assert!(rust_plugin.enabled);
        assert!(rust_plugin
            .file_patterns
            .extensions
            .contains(&".rs".to_string()));
        assert!(rust_plugin
            .file_patterns
            .filenames
            .contains(&"Cargo.toml".to_string()));
    }

    #[tokio::test]
    async fn test_config_save_and_load() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("test_config.yaml");

        // Create and save a config
        let original_config = create_test_config_with_plugins();
        original_config
            .save(&config_path)
            .await
            .expect("Failed to save config");

        // Verify file was created
        assert!(config_path.exists());

        // Load the config back
        let loaded_config = Config::load(&config_path)
            .await
            .expect("Failed to load config");

        // Verify the loaded config matches the original
        assert_eq!(loaded_config.output_dir, original_config.output_dir);
        assert_eq!(loaded_config.llm.provider, original_config.llm.provider);
        assert_eq!(loaded_config.llm.model, original_config.llm.model);
        assert_eq!(
            loaded_config.python_executable,
            original_config.python_executable
        );

        // Verify plugins were preserved
        assert_eq!(loaded_config.plugins.len(), original_config.plugins.len());
        assert!(loaded_config.plugins.contains_key("javascript"));

        let js_plugin = &loaded_config.plugins["javascript"];
        assert!(js_plugin.enabled);
        assert!(js_plugin
            .file_patterns
            .extensions
            .contains(&".js".to_string()));
    }

    #[tokio::test]
    async fn test_config_load_from_minimal_yaml() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("minimal_config.yaml");

        // Write minimal YAML config
        let yaml_content = create_minimal_yaml_config();
        fs::write(&config_path, yaml_content)
            .await
            .expect("Failed to write test config");

        // Load the config
        let config = Config::load(&config_path)
            .await
            .expect("Failed to load minimal config");

        // Verify it loaded correctly
        assert_eq!(config.output_dir, "test_output");
        assert_eq!(config.python_executable, Some("python3".to_string()));
        assert_eq!(config.llm.model, "test-model");
        assert_eq!(config.llm.timeout_seconds, 60);
        assert_eq!(config.scanning.max_file_size_mb, 5);

        // Verify plugins
        assert!(config.plugins.contains_key("python"));
        let python_plugin = &config.plugins["python"];
        assert!(python_plugin.enabled);
    }

    #[tokio::test]
    async fn test_config_load_nonexistent_file() {
        let nonexistent_path = PathBuf::from("/definitely/does/not/exist/config.yaml");

        let result = Config::load(&nonexistent_path).await;
        assert!(result.is_err());

        let error_message = format!("{}", result.unwrap_err());
        assert!(error_message.contains("No such file") || error_message.contains("cannot find"));
    }

    #[tokio::test]
    async fn test_config_load_invalid_yaml() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("invalid_config.yaml");

        // Write invalid YAML
        let invalid_yaml = r#"
output_dir: "test"
invalid_yaml: [unclosed bracket
plugins:
  - this is not: valid yaml structure
"#;
        fs::write(&config_path, invalid_yaml)
            .await
            .expect("Failed to write invalid config");

        let result = Config::load(&config_path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_config_save_to_readonly_location() {
        // Try to save to a location that should fail (root directory)
        let readonly_path = PathBuf::from("/root/readonly_config.yaml");
        let config = Config::default();

        let result = config.save(&readonly_path).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_find_plugin_for_file_by_extension() {
        let config = create_test_config_with_plugins();

        // Test Python files
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("src/main.py")),
            Some("python".to_string())
        );
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("script.py")),
            Some("python".to_string())
        );

        // Test Rust files
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("src/main.rs")),
            Some("rust".to_string())
        );

        // Test JavaScript files (from our custom plugin)
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("app.js")),
            Some("javascript".to_string())
        );
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("component.jsx")),
            Some("javascript".to_string())
        );

        // Test unknown extension
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("README.md")),
            None
        );
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("image.png")),
            None
        );
    }

    #[test]
    fn test_find_plugin_for_file_by_filename() {
        let config = create_test_config_with_plugins();

        // Test Python ecosystem files
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("requirements.txt")),
            Some("python".to_string())
        );
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("setup.py")),
            Some("python".to_string())
        );
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("pyproject.toml")),
            Some("python".to_string())
        );

        // Test Rust ecosystem files
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("Cargo.toml")),
            Some("rust".to_string())
        );
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("Cargo.lock")),
            Some("rust".to_string())
        );

        // Test JavaScript ecosystem files (from our custom plugin)
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("package.json")),
            Some("javascript".to_string())
        );
    }

    #[test]
    fn test_find_plugin_for_file_case_sensitivity() {
        let config = create_test_config_with_plugins();

        // Test case variations
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("REQUIREMENTS.TXT")),
            Some("python".to_string())
        );
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("Requirements.txt")),
            Some("python".to_string())
        );
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("cargo.toml")),
            Some("rust".to_string())
        );
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("CARGO.TOML")),
            Some("rust".to_string())
        );
    }

    #[test]
    fn test_find_plugin_for_file_with_path() {
        let config = create_test_config_with_plugins();

        // Test files in subdirectories
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("src/utils/helper.py")),
            Some("python".to_string())
        );
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("backend/core/main.rs")),
            Some("rust".to_string())
        );
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("frontend/src/app.js")),
            Some("javascript".to_string())
        );

        // Test absolute paths
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("/home/user/project/script.py")),
            Some("python".to_string())
        );
    }

    #[test]
    fn test_find_plugin_for_file_disabled_plugin() {
        let mut config = create_test_config_with_plugins();

        // Disable the JavaScript plugin
        config.plugins.get_mut("javascript").unwrap().enabled = false;

        // Should not match disabled plugin
        assert_eq!(config.find_plugin_for_file(&PathBuf::from("app.js")), None);
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("package.json")),
            None
        );

        // But enabled plugins should still work
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("main.py")),
            Some("python".to_string())
        );
    }

    #[test]
    fn test_find_plugin_for_file_no_extension() {
        let config = create_test_config_with_plugins();

        // Files without extensions should not match extension patterns
        assert_eq!(config.find_plugin_for_file(&PathBuf::from("README")), None);
        assert_eq!(config.find_plugin_for_file(&PathBuf::from("LICENSE")), None);
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("Makefile")),
            None
        );

        // But should match filename patterns
        assert_eq!(
            config.find_plugin_for_file(&PathBuf::from("requirements.txt")),
            Some("python".to_string())
        );
    }

    #[test]
    fn test_plugin_source_types() {
        let config = Config::default();

        // Test that default plugins use builtin source
        let python_plugin = &config.plugins["python"];
        match &python_plugin.source {
            PluginSource::Builtin { name } => {
                assert_eq!(name, "python");
            }
            _ => panic!("Expected builtin source for python plugin"),
        }

        let rust_plugin = &config.plugins["rust"];
        match &rust_plugin.source {
            PluginSource::Builtin { name } => {
                assert_eq!(name, "rust");
            }
            _ => panic!("Expected builtin source for rust plugin"),
        }
    }

    #[test]
    fn test_file_patterns_structure() {
        let config = Config::default();
        let python_plugin = &config.plugins["python"];

        // Test that file patterns are properly structured
        assert!(!python_plugin.file_patterns.extensions.is_empty());
        assert!(!python_plugin.file_patterns.filenames.is_empty());
        assert!(python_plugin.file_patterns.glob_patterns.is_some());

        // Test specific patterns
        let patterns = &python_plugin.file_patterns;
        assert!(patterns.extensions.contains(&".py".to_string()));
        assert!(patterns.filenames.contains(&"requirements.txt".to_string()));

        if let Some(ref globs) = patterns.glob_patterns {
            assert!(globs.iter().any(|p| p.contains("requirements")));
        }
    }

    #[tokio::test]
    async fn test_config_roundtrip_preserves_data() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("roundtrip_config.yaml");

        // Create a config with specific values
        let original_config = Config {
            output_dir: "custom_output".to_string(),
            python_executable: Some("/usr/bin/python3.11".to_string()),
            llm: LlmConfig {
                timeout_seconds: 120,
                ..Config::default().llm
            },
            scanning: ScanConfig {
                max_file_size_mb: 25,
                ..Config::default().scanning
            },
            ..Config::default()
        };

        // Save and reload
        original_config
            .save(&config_path)
            .await
            .expect("Failed to save");
        let loaded_config = Config::load(&config_path).await.expect("Failed to load");

        // Verify all custom values are preserved
        assert_eq!(loaded_config.output_dir, "custom_output");
        assert_eq!(
            loaded_config.python_executable,
            Some("/usr/bin/python3.11".to_string())
        );
        assert_eq!(loaded_config.llm.timeout_seconds, 120);
        assert_eq!(loaded_config.scanning.max_file_size_mb, 25);

        // Verify plugin data is preserved
        assert_eq!(loaded_config.plugins.len(), original_config.plugins.len());
        for (key, original_plugin) in &original_config.plugins {
            let loaded_plugin = loaded_config.plugins.get(key).expect("Plugin missing");
            assert_eq!(loaded_plugin.enabled, original_plugin.enabled);
            assert_eq!(
                loaded_plugin.file_patterns.extensions.len(),
                original_plugin.file_patterns.extensions.len()
            );
        }
    }
}
