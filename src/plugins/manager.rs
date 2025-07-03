use crate::utils::config::{Config, PluginSource};
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug)]
pub struct PluginInfo {
    pub name: String,
    pub path: PathBuf,
    pub plugin_type: String,       // "input" or "output"
    pub extensions: Vec<String>,   // For input plugins
    pub filenames: Vec<String>,    // For input plugins
    pub output_types: Vec<String>, // For output plugins
    pub formats: Vec<String>,      // For output plugins
    pub source: PluginSource,
    pub enabled: bool,
}

pub struct PluginManager {
    config: Config,
}

impl PluginManager {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn discover_plugins(&self) -> Result<Vec<PluginInfo>> {
        let mut plugins = Vec::new();

        // Discover input plugins
        for (name, plugin_config) in &self.config.input_plugins {
            if !plugin_config.enabled {
                continue;
            }

            let path = self
                .resolve_plugin_path(name, &plugin_config.source, "input")
                .await?;

            plugins.push(PluginInfo {
                name: name.clone(),
                path,
                plugin_type: "input".to_string(),
                extensions: plugin_config.file_patterns.extensions.clone(),
                filenames: plugin_config.file_patterns.filenames.clone(),
                output_types: vec![], // Input plugins don't have output types
                formats: vec![],      // Input plugins don't have formats
                source: plugin_config.source.clone(),
                enabled: plugin_config.enabled,
            });
        }

        // Discover output plugins
        for (name, plugin_config) in &self.config.output_plugins {
            if !plugin_config.enabled {
                continue;
            }

            let path = self
                .resolve_plugin_path(name, &plugin_config.source, "output")
                .await?;

            plugins.push(PluginInfo {
                name: name.clone(),
                path,
                plugin_type: "output".to_string(),
                extensions: vec![], // Output plugins don't analyze files
                filenames: vec![],  // Output plugins don't analyze files
                output_types: plugin_config.output_types.clone(),
                formats: plugin_config.formats.clone(),
                source: plugin_config.source.clone(),
                enabled: plugin_config.enabled,
            });
        }

        Ok(plugins)
    }

    /// Discover only input plugins
    pub async fn discover_input_plugins(&self) -> Result<Vec<PluginInfo>> {
        let all_plugins = self.discover_plugins().await?;
        Ok(all_plugins
            .into_iter()
            .filter(|p| p.plugin_type == "input")
            .collect())
    }

    /// Discover only output plugins
    pub async fn discover_output_plugins(&self) -> Result<Vec<PluginInfo>> {
        let all_plugins = self.discover_plugins().await?;
        Ok(all_plugins
            .into_iter()
            .filter(|p| p.plugin_type == "output")
            .collect())
    }

    /// Find input plugins that can handle a specific file
    pub async fn find_input_plugins_for_file(
        &self,
        file_path: &std::path::Path,
    ) -> Result<Vec<PluginInfo>> {
        let input_plugins = self.discover_input_plugins().await?;
        let mut matching_plugins = Vec::new();

        let filename = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!(".{}", ext.to_lowercase()));

        for plugin in input_plugins {
            // Check extensions
            if let Some(ref ext) = extension {
                if plugin.extensions.contains(ext) {
                    matching_plugins.push(plugin);
                    continue;
                }
            }

            // Check exact filenames
            if plugin
                .filenames
                .iter()
                .any(|pattern| pattern.to_lowercase() == filename.to_lowercase())
            {
                matching_plugins.push(plugin);
            }
        }

        Ok(matching_plugins)
    }

    /// Find output plugins that can generate specific output type and format
    pub async fn find_output_plugins_for_generation(
        &self,
        output_type: &str,
        format: &str,
    ) -> Result<Vec<PluginInfo>> {
        let output_plugins = self.discover_output_plugins().await?;
        let mut matching_plugins = Vec::new();

        for plugin in output_plugins {
            // Check if plugin supports the output type
            let supports_type = plugin.output_types.is_empty()
                || plugin.output_types.contains(&output_type.to_string());

            // Check if plugin supports the format
            let supports_format =
                plugin.formats.is_empty() || plugin.formats.contains(&format.to_string());

            if supports_type && supports_format {
                matching_plugins.push(plugin);
            }
        }

        Ok(matching_plugins)
    }

    /// Get plugin by name and type
    pub async fn get_plugin(&self, name: &str, plugin_type: &str) -> Result<Option<PluginInfo>> {
        let plugins = self.discover_plugins().await?;
        Ok(plugins
            .into_iter()
            .find(|p| p.name == name && p.plugin_type == plugin_type))
    }

    /// Check if a plugin exists and is enabled
    pub fn is_plugin_enabled(&self, name: &str, plugin_type: &str) -> bool {
        match plugin_type {
            "input" => self
                .config
                .input_plugins
                .get(name)
                .map(|config| config.enabled)
                .unwrap_or(false),
            "output" => self
                .config
                .output_plugins
                .get(name)
                .map(|config| config.enabled)
                .unwrap_or(false),
            _ => false,
        }
    }

    /// Get plugin statistics
    pub async fn get_plugin_stats(&self) -> Result<PluginStats> {
        let all_plugins = self.discover_plugins().await?;

        let total_plugins = all_plugins.len();
        let enabled_plugins = all_plugins.iter().filter(|p| p.enabled).count();
        let input_plugins = all_plugins
            .iter()
            .filter(|p| p.plugin_type == "input")
            .count();
        let output_plugins = all_plugins
            .iter()
            .filter(|p| p.plugin_type == "output")
            .count();
        let enabled_input = all_plugins
            .iter()
            .filter(|p| p.plugin_type == "input" && p.enabled)
            .count();
        let enabled_output = all_plugins
            .iter()
            .filter(|p| p.plugin_type == "output" && p.enabled)
            .count();

        // Count by source type
        let mut builtin_count = 0;
        let mut local_count = 0;
        let mut remote_count = 0;

        for plugin in &all_plugins {
            match plugin.source {
                PluginSource::Builtin { .. } => builtin_count += 1,
                PluginSource::Local { .. } => local_count += 1,
                PluginSource::GitHub { .. } | PluginSource::Git { .. } => remote_count += 1,
            }
        }

        Ok(PluginStats {
            total_plugins,
            enabled_plugins,
            input_plugins,
            output_plugins,
            enabled_input,
            enabled_output,
            builtin_plugins: builtin_count,
            local_plugins: local_count,
            remote_plugins: remote_count,
        })
    }

    async fn resolve_plugin_path(
        &self,
        _name: &str,
        source: &PluginSource,
        plugin_type: &str,
    ) -> Result<PathBuf> {
        match source {
            PluginSource::Local { path } => Ok(PathBuf::from(path)),
            PluginSource::Builtin { name: plugin_name } => {
                // Built-in plugins are organized by type
                match plugin_type {
                    "input" => Ok(PathBuf::from(format!(
                        "plugins/input/{plugin_name}_analyzer.py"
                    ))),
                    "output" => Ok(PathBuf::from(format!("plugins/output/{plugin_name}.py"))),
                    _ => Err(anyhow::anyhow!("Unknown plugin type: {plugin_type}")),
                }
            }
            PluginSource::GitHub { repo, version } => {
                // TODO: Implement GitHub plugin downloading
                let version_str = version.as_deref().unwrap_or("latest");
                Ok(PathBuf::from(format!(
                    ".csd_cache/github/{repo}/{version_str}/{_name}.py"
                )))
            }
            PluginSource::Git { url, branch } => {
                // TODO: Implement Git plugin cloning
                let branch_str = branch.as_deref().unwrap_or("main");
                Ok(PathBuf::from(format!(
                    ".csd_cache/git/{}/{branch_str}/{_name}.py",
                    url.replace('/', "_")
                )))
            }
        }
    }

    /// Install a plugin from a remote source
    pub async fn install_plugin(
        &mut self,
        name: String,
        _source: PluginSource,
        plugin_type: String,
    ) -> Result<()> {
        // TODO: Implement plugin installation
        // This would download/clone the plugin and add it to configuration

        match plugin_type.as_str() {
            "input" => {
                // Would need to determine file patterns from plugin
                println!("Installing input plugin '{name}' (not yet implemented)");
            }
            "output" => {
                // Would need to determine output types and formats from plugin
                println!("Installing output plugin '{name}' (not yet implemented)");
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown plugin type: {}", plugin_type));
            }
        }

        Ok(())
    }

    /// Remove a plugin
    pub fn remove_plugin(&mut self, name: &str, plugin_type: &str) -> Result<bool> {
        let removed = match plugin_type {
            "input" => self.config.remove_input_plugin(name).is_some(),
            "output" => self.config.remove_output_plugin(name).is_some(),
            _ => return Err(anyhow::anyhow!("Unknown plugin type: {}", plugin_type)),
        };

        Ok(removed)
    }

    /// Enable or disable a plugin
    pub fn set_plugin_enabled(
        &mut self,
        name: &str,
        plugin_type: &str,
        enabled: bool,
    ) -> Result<()> {
        match plugin_type {
            "input" => {
                if let Some(plugin_config) = self.config.input_plugins.get_mut(name) {
                    plugin_config.enabled = enabled;
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Input plugin '{}' not found", name))
                }
            }
            "output" => {
                if let Some(plugin_config) = self.config.output_plugins.get_mut(name) {
                    plugin_config.enabled = enabled;
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("Output plugin '{}' not found", name))
                }
            }
            _ => Err(anyhow::anyhow!("Unknown plugin type: {}", plugin_type)),
        }
    }

    /// Validate that all enabled plugins are available
    pub async fn validate_plugins(&self) -> Result<ValidationResult> {
        let mut result = ValidationResult::default();

        // Validate input plugins
        for (name, config) in &self.config.input_plugins {
            if !config.enabled {
                continue;
            }

            let path = self
                .resolve_plugin_path(name, &config.source, "input")
                .await?;

            if path.exists() {
                result.valid_plugins.push(format!("input:{name}"));
            } else {
                result
                    .invalid_plugins
                    .push(format!("input:{name} (path: {})", path.display()));
            }
        }

        // Validate output plugins
        for (name, config) in &self.config.output_plugins {
            if !config.enabled {
                continue;
            }

            let path = self
                .resolve_plugin_path(name, &config.source, "output")
                .await?;

            if path.exists() {
                result.valid_plugins.push(format!("output:{name}"));
            } else {
                result
                    .invalid_plugins
                    .push(format!("output:{name} (path: {})", path.display()));
            }
        }

        Ok(result)
    }
}

#[derive(Debug, Default)]
pub struct PluginStats {
    pub total_plugins: usize,
    pub enabled_plugins: usize,
    pub input_plugins: usize,
    pub output_plugins: usize,
    pub enabled_input: usize,
    pub enabled_output: usize,
    pub builtin_plugins: usize,
    pub local_plugins: usize,
    pub remote_plugins: usize,
}

#[derive(Debug, Default)]
pub struct ValidationResult {
    pub valid_plugins: Vec<String>,
    pub invalid_plugins: Vec<String>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        self.invalid_plugins.is_empty()
    }

    pub fn has_issues(&self) -> bool {
        !self.invalid_plugins.is_empty()
    }
}
