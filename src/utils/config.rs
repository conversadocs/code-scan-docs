use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub output_dir: String,
    pub llm: LlmConfig,
    pub scanning: ScanConfig,
    pub input_plugins: HashMap<String, InputPluginConfig>, // NEW: Separated plugin types
    pub output_plugins: HashMap<String, OutputPluginConfig>, // NEW: Output plugins
    pub python_executable: Option<String>,

    // Legacy field for backward compatibility
    #[serde(default)]
    pub plugins: Option<HashMap<String, LegacyPluginConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputPluginConfig {
    pub source: PluginSource,
    pub file_patterns: FilePatterns,
    pub enabled: bool,
    pub config: Option<serde_yaml::Value>, // Plugin-specific configuration
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPluginConfig {
    pub source: PluginSource,
    pub output_types: Vec<String>, // e.g., ["documentation", "quality_report"]
    pub formats: Vec<String>,      // e.g., ["markdown", "html", "pdf"]
    pub enabled: bool,
    pub config: Option<serde_yaml::Value>, // Plugin-specific configuration
}

// Legacy plugin config for backward compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyPluginConfig {
    pub source: PluginSource,
    pub file_patterns: Option<FilePatterns>,
    pub output_types: Option<Vec<String>>,
    pub formats: Option<Vec<String>>,
    pub enabled: bool,
    pub config: Option<serde_yaml::Value>,
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
    Builtin {
        name: String,
        plugin_type: String, // NEW: Separate plugin type field
    },
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
        let mut input_plugins = HashMap::new();
        let mut output_plugins = HashMap::new();

        // Built-in Python input plugin for self-analysis
        input_plugins.insert(
            "python".to_string(),
            InputPluginConfig {
                source: PluginSource::Builtin {
                    name: "python_analyzer".to_string(),
                    plugin_type: "code".to_string(),
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

        // Built-in Rust input plugin for self-analysis
        input_plugins.insert(
            "rust".to_string(),
            InputPluginConfig {
                source: PluginSource::Builtin {
                    name: "rust_analyzer".to_string(),
                    plugin_type: "code".to_string(),
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

        // Built-in Markdown documentation output plugin
        output_plugins.insert(
            "markdown_docs".to_string(),
            OutputPluginConfig {
                source: PluginSource::Builtin {
                    name: "markdown_docs".to_string(),
                    plugin_type: "docs".to_string(),
                },
                output_types: vec!["documentation".to_string()],
                formats: vec!["markdown".to_string()],
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
                    ".csd_cache/".to_string(),
                ],
                include_hidden: false,
                max_file_size_mb: 10,
            },
            input_plugins,
            output_plugins,
            python_executable: None,
            plugins: None, // Legacy field
        }
    }
}

impl Config {
    pub async fn load(path: &Path) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut config: Config = serde_yaml::from_str(&content)?;

        // Handle legacy configuration migration
        config.migrate_legacy_plugins();

        Ok(config)
    }

    pub async fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Migrate legacy plugin configuration to new typed structure
    fn migrate_legacy_plugins(&mut self) {
        if let Some(legacy_plugins) = &self.plugins {
            for (name, legacy_config) in legacy_plugins {
                // Try to determine if it's an input or output plugin based on configuration
                if legacy_config.file_patterns.is_some() {
                    // Has file patterns, likely an input plugin
                    let input_config = InputPluginConfig {
                        source: legacy_config.source.clone(),
                        file_patterns: legacy_config.file_patterns.clone().unwrap_or_else(|| {
                            FilePatterns {
                                extensions: vec![],
                                filenames: vec![],
                                glob_patterns: None,
                            }
                        }),
                        enabled: legacy_config.enabled,
                        config: legacy_config.config.clone(),
                    };
                    self.input_plugins.insert(name.clone(), input_config);
                } else if legacy_config.output_types.is_some() || legacy_config.formats.is_some() {
                    // Has output types or formats, likely an output plugin
                    let output_config = OutputPluginConfig {
                        source: legacy_config.source.clone(),
                        output_types: legacy_config.output_types.clone().unwrap_or_default(),
                        formats: legacy_config.formats.clone().unwrap_or_default(),
                        enabled: legacy_config.enabled,
                        config: legacy_config.config.clone(),
                    };
                    self.output_plugins.insert(name.clone(), output_config);
                }
            }

            // Clear legacy plugins after migration
            self.plugins = None;
        }
    }

    /// Find which input plugin should handle a given file
    pub fn find_input_plugin_for_file(&self, file_path: &Path) -> Option<String> {
        let filename = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!(".{}", ext.to_lowercase()));

        for (plugin_name, plugin_config) in &self.input_plugins {
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

    /// Find output plugins that can generate the specified output type and format
    pub fn find_output_plugins_for_type(&self, output_type: &str, format: &str) -> Vec<String> {
        let mut matching_plugins = Vec::new();

        for (plugin_name, plugin_config) in &self.output_plugins {
            if !plugin_config.enabled {
                continue;
            }

            // Check if plugin supports the output type
            let supports_type = plugin_config.output_types.is_empty()
                || plugin_config
                    .output_types
                    .contains(&output_type.to_string());

            // Check if plugin supports the format
            let supports_format = plugin_config.formats.is_empty()
                || plugin_config.formats.contains(&format.to_string());

            if supports_type && supports_format {
                matching_plugins.push(plugin_name.clone());
            }
        }

        matching_plugins
    }

    /// Get all enabled input plugins
    pub fn get_enabled_input_plugins(&self) -> Vec<(&String, &InputPluginConfig)> {
        self.input_plugins
            .iter()
            .filter(|(_, config)| config.enabled)
            .collect()
    }

    /// Get all enabled output plugins
    pub fn get_enabled_output_plugins(&self) -> Vec<(&String, &OutputPluginConfig)> {
        self.output_plugins
            .iter()
            .filter(|(_, config)| config.enabled)
            .collect()
    }

    /// Get input plugin configuration by name
    pub fn get_input_plugin(&self, name: &str) -> Option<&InputPluginConfig> {
        self.input_plugins.get(name)
    }

    /// Get output plugin configuration by name
    pub fn get_output_plugin(&self, name: &str) -> Option<&OutputPluginConfig> {
        self.output_plugins.get(name)
    }

    /// Add or update an input plugin
    pub fn add_input_plugin(&mut self, name: String, config: InputPluginConfig) {
        self.input_plugins.insert(name, config);
    }

    /// Add or update an output plugin
    pub fn add_output_plugin(&mut self, name: String, config: OutputPluginConfig) {
        self.output_plugins.insert(name, config);
    }

    /// Remove an input plugin
    pub fn remove_input_plugin(&mut self, name: &str) -> Option<InputPluginConfig> {
        self.input_plugins.remove(name)
    }

    /// Remove an output plugin
    pub fn remove_output_plugin(&mut self, name: &str) -> Option<OutputPluginConfig> {
        self.output_plugins.remove(name)
    }

    /// Legacy compatibility method
    pub fn find_plugin_for_file(&self, file_path: &Path) -> Option<String> {
        self.find_input_plugin_for_file(file_path)
    }

    /// Get summary of all plugin configurations
    pub fn get_plugin_summary(&self) -> PluginSummary {
        PluginSummary {
            total_input_plugins: self.input_plugins.len(),
            enabled_input_plugins: self.input_plugins.values().filter(|c| c.enabled).count(),
            total_output_plugins: self.output_plugins.len(),
            enabled_output_plugins: self.output_plugins.values().filter(|c| c.enabled).count(),
            input_plugin_names: self.input_plugins.keys().cloned().collect(),
            output_plugin_names: self.output_plugins.keys().cloned().collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PluginSummary {
    pub total_input_plugins: usize,
    pub enabled_input_plugins: usize,
    pub total_output_plugins: usize,
    pub enabled_output_plugins: usize,
    pub input_plugin_names: Vec<String>,
    pub output_plugin_names: Vec<String>,
}
