use crate::utils::config::{Config, PluginSource};
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug)]
pub struct PluginInfo {
    pub name: String,
    pub path: PathBuf,
    pub extensions: Vec<String>,
    pub filenames: Vec<String>,
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

        for (name, plugin_config) in &self.config.plugins {
            if !plugin_config.enabled {
                continue;
            }

            let path = self
                .resolve_plugin_path(name, &plugin_config.source)
                .await?;

            plugins.push(PluginInfo {
                name: name.clone(),
                path,
                extensions: plugin_config.file_patterns.extensions.clone(),
                filenames: plugin_config.file_patterns.filenames.clone(),
                source: plugin_config.source.clone(),
                enabled: plugin_config.enabled,
            });
        }

        Ok(plugins)
    }

    async fn resolve_plugin_path(&self, _name: &str, source: &PluginSource) -> Result<PathBuf> {
        match source {
            PluginSource::Local { path } => Ok(PathBuf::from(path)),
            PluginSource::Builtin { name: plugin_name } => {
                // Built-in plugins are in the plugins/ directory
                Ok(PathBuf::from(format!("plugins/{plugin_name}_analyzer.py")))
            }
            PluginSource::GitHub { repo, version } => {
                // TODO: Implement GitHub plugin downloading
                // For now, assume they're in a cache directory
                let version_str = version.as_deref().unwrap_or("latest");
                Ok(PathBuf::from(format!(
                    ".csd_cache/github/{repo}/{version_str}/plugin.py"
                )))
            }
            PluginSource::Git { url, branch } => {
                // TODO: Implement Git plugin cloning
                let branch_str = branch.as_deref().unwrap_or("main");
                Ok(PathBuf::from(format!(
                    ".csd_cache/git/{}/{}/plugin.py",
                    url.replace('/', "_"),
                    branch_str
                )))
            }
        }
    }
}
