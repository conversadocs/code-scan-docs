use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::collections::HashMap;

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
    GitHub { repo: String, version: Option<String> },
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
        plugins.insert("python".to_string(), PluginConfig {
            source: PluginSource::Builtin { name: "python".to_string() },
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
        });

        // Built-in Rust plugin for self-analysis
        plugins.insert("rust".to_string(), PluginConfig {
            source: PluginSource::Builtin { name: "rust".to_string() },
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
        });

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
                    ".csd_cache/".to_string(),  // Our own cache directory
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
        let filename = file_path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        let extension = file_path.extension()
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
            if plugin_config.file_patterns.filenames.iter()
                .any(|pattern| pattern.to_lowercase() == filename.to_lowercase()) {
                return Some(plugin_name.clone());
            }

            // TODO: Check glob patterns if needed
        }

        None
    }
}
