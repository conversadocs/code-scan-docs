//! Configuration management for the application.
//!
//! This module provides comprehensive configuration handling including plugin
//! configuration, scanning settings, LLM integration settings, and legacy
//! configuration migration capabilities.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Main application configuration structure.
///
/// Contains all configuration settings for the application including plugin
/// configurations, scanning parameters, LLM settings, and output preferences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub output_dir: String,
    pub llm: LlmConfig,
    pub scanning: ScanConfig,
    pub input_plugins: HashMap<String, InputPluginConfig>,
    pub output_plugins: HashMap<String, OutputPluginConfig>,
    pub python_executable: Option<String>,

    #[serde(default)]
    pub plugins: Option<HashMap<String, LegacyPluginConfig>>,
}

/// Configuration for an input (analysis) plugin.
///
/// Defines how an input plugin should be loaded, what files it can handle,
/// and any plugin-specific configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputPluginConfig {
    pub source: PluginSource,
    pub file_patterns: FilePatterns,
    pub enabled: bool,
    pub config: Option<serde_yaml::Value>,
}

/// Configuration for an output (generation) plugin.
///
/// Defines how an output plugin should be loaded, what output types and formats
/// it supports, and any plugin-specific configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPluginConfig {
    pub source: PluginSource,
    pub output_types: Vec<String>,
    pub formats: Vec<String>,
    pub enabled: bool,
    pub config: Option<serde_yaml::Value>,
}

/// Legacy plugin configuration format for backward compatibility.
///
/// Supports older configuration files that used a unified plugin format
/// before the separation of input and output plugins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyPluginConfig {
    pub source: PluginSource,
    pub file_patterns: Option<FilePatterns>,
    pub output_types: Option<Vec<String>>,
    pub formats: Option<Vec<String>>,
    pub enabled: bool,
    pub config: Option<serde_yaml::Value>,
}

/// File matching patterns for input plugins.
///
/// Defines which files an input plugin can analyze based on file extensions,
/// specific filenames, and glob patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePatterns {
    pub extensions: Vec<String>,
    pub filenames: Vec<String>,
    pub glob_patterns: Option<Vec<String>>,
}

/// Source location and type for plugins.
///
/// Defines where plugins are located and how they should be loaded,
/// supporting local files, remote repositories, and built-in plugins.
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
    Builtin { name: String, plugin_type: String },
}

/// Configuration for Large Language Model integration.
///
/// Contains settings for connecting to and using LLM services for
/// enhanced analysis and documentation generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub timeout_seconds: u64,
}

/// Configuration for project scanning behavior.
///
/// Controls which files are included in scanning, size limits,
/// and ignore patterns for project analysis.
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
            plugins: None,
        }
    }
}

impl Config {
    /// Loads configuration from a YAML file.
    ///
    /// Reads and parses a configuration file, automatically migrating
    /// legacy configuration formats if necessary.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    ///
    /// The loaded configuration or an error if loading fails.
    pub async fn load(path: &Path) -> Result<Self> {
        let content = tokio::fs::read_to_string(path).await?;
        let mut config: Config = serde_yaml::from_str(&content)?;

        config.migrate_legacy_plugins();

        Ok(config)
    }

    /// Saves the configuration to a YAML file.
    ///
    /// Serializes the configuration and writes it to the specified path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the configuration should be saved
    ///
    /// # Returns
    ///
    /// Result indicating success or failure of the save operation.
    pub async fn save(&self, path: &Path) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Migrates legacy plugin configurations to the new format.
    ///
    /// Converts older unified plugin configurations into separate input
    /// and output plugin configurations, removing the legacy format.
    fn migrate_legacy_plugins(&mut self) {
        if let Some(legacy_plugins) = &self.plugins {
            for (name, legacy_config) in legacy_plugins {
                if legacy_config.file_patterns.is_some() {
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

            self.plugins = None;
        }
    }

    /// Finds the appropriate input plugin for analyzing a specific file.
    ///
    /// Matches the file's extension and name against plugin file patterns
    /// to determine which plugin should handle the file.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to find a plugin for
    ///
    /// # Returns
    ///
    /// Name of the matching plugin, or None if no plugin can handle the file.
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

            if let Some(ref ext) = extension {
                if plugin_config.file_patterns.extensions.contains(ext) {
                    return Some(plugin_name.clone());
                }
            }

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

    /// Finds output plugins that can generate specific output types and formats.
    ///
    /// # Arguments
    ///
    /// * `output_type` - Type of output to generate (e.g., "documentation")
    /// * `format` - Output format (e.g., "markdown", "html")
    ///
    /// # Returns
    ///
    /// Vector of plugin names that can generate the requested output.
    pub fn find_output_plugins_for_type(&self, output_type: &str, format: &str) -> Vec<String> {
        let mut matching_plugins = Vec::new();

        for (plugin_name, plugin_config) in &self.output_plugins {
            if !plugin_config.enabled {
                continue;
            }

            let supports_type = plugin_config.output_types.is_empty()
                || plugin_config
                    .output_types
                    .contains(&output_type.to_string());

            let supports_format = plugin_config.formats.is_empty()
                || plugin_config.formats.contains(&format.to_string());

            if supports_type && supports_format {
                matching_plugins.push(plugin_name.clone());
            }
        }

        matching_plugins
    }

    /// Returns all enabled input plugins and their configurations.
    ///
    /// # Returns
    ///
    /// Vector of tuples containing plugin names and their configurations.
    pub fn get_enabled_input_plugins(&self) -> Vec<(&String, &InputPluginConfig)> {
        self.input_plugins
            .iter()
            .filter(|(_, config)| config.enabled)
            .collect()
    }

    /// Returns all enabled output plugins and their configurations.
    ///
    /// # Returns
    ///
    /// Vector of tuples containing plugin names and their configurations.
    pub fn get_enabled_output_plugins(&self) -> Vec<(&String, &OutputPluginConfig)> {
        self.output_plugins
            .iter()
            .filter(|(_, config)| config.enabled)
            .collect()
    }

    /// Retrieves configuration for a specific input plugin.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the input plugin
    ///
    /// # Returns
    ///
    /// Plugin configuration if found, or None if the plugin doesn't exist.
    pub fn get_input_plugin(&self, name: &str) -> Option<&InputPluginConfig> {
        self.input_plugins.get(name)
    }

    /// Retrieves configuration for a specific output plugin.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the output plugin
    ///
    /// # Returns
    ///
    /// Plugin configuration if found, or None if the plugin doesn't exist.
    pub fn get_output_plugin(&self, name: &str) -> Option<&OutputPluginConfig> {
        self.output_plugins.get(name)
    }

    /// Adds a new input plugin configuration.
    ///
    /// # Arguments
    ///
    /// * `name` - Name for the plugin
    /// * `config` - Plugin configuration
    pub fn add_input_plugin(&mut self, name: String, config: InputPluginConfig) {
        self.input_plugins.insert(name, config);
    }

    /// Adds a new output plugin configuration.
    ///
    /// # Arguments
    ///
    /// * `name` - Name for the plugin
    /// * `config` - Plugin configuration
    pub fn add_output_plugin(&mut self, name: String, config: OutputPluginConfig) {
        self.output_plugins.insert(name, config);
    }

    /// Removes an input plugin configuration.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the plugin to remove
    ///
    /// # Returns
    ///
    /// The removed plugin configuration, or None if it wasn't found.
    pub fn remove_input_plugin(&mut self, name: &str) -> Option<InputPluginConfig> {
        self.input_plugins.remove(name)
    }

    /// Removes an output plugin configuration.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the plugin to remove
    ///
    /// # Returns
    ///
    /// The removed plugin configuration, or None if it wasn't found.
    pub fn remove_output_plugin(&mut self, name: &str) -> Option<OutputPluginConfig> {
        self.output_plugins.remove(name)
    }

    /// Alias for `find_input_plugin_for_file` for backward compatibility.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the file to find a plugin for
    ///
    /// # Returns
    ///
    /// Name of the matching plugin, or None if no plugin can handle the file.
    pub fn find_plugin_for_file(&self, file_path: &Path) -> Option<String> {
        self.find_input_plugin_for_file(file_path)
    }

    /// Generates a summary of the current plugin configuration.
    ///
    /// Provides counts and lists of configured plugins for reporting
    /// and monitoring purposes.
    ///
    /// # Returns
    ///
    /// Summary statistics about the plugin configuration.
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

/// Summary statistics about plugin configuration.
///
/// Contains counts of total and enabled plugins by type, as well as
/// lists of plugin names for detailed reporting.
#[derive(Debug, Clone)]
pub struct PluginSummary {
    pub total_input_plugins: usize,
    pub enabled_input_plugins: usize,
    pub total_output_plugins: usize,
    pub enabled_output_plugins: usize,
    pub input_plugin_names: Vec<String>,
    pub output_plugin_names: Vec<String>,
}
