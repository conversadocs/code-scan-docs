use anyhow::{Context, Result};
use log::{debug, error, warn};
use serde_json;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::fs;
use tokio::process::Command;
use uuid::Uuid;

use crate::plugins::interface::{
    InputPluginInterface, OutputPluginInput, OutputPluginInterface, OutputPluginResult, PluginInfo,
    PluginInput, PluginInterface, PluginMessage, PluginResponse, PluginType,
};

/// Base plugin communicator with common functionality
pub struct PluginCommunicator {
    plugin_path: PathBuf,
    python_executable: String,
    cache_dir: PathBuf,
}

impl PluginCommunicator {
    pub fn new(plugin_path: PathBuf) -> Self {
        let cache_dir = PathBuf::from(".csd_cache");

        Self {
            plugin_path,
            python_executable: "python".to_string(),
            cache_dir,
        }
    }

    pub fn with_cache_dir(mut self, cache_dir: PathBuf) -> Self {
        self.cache_dir = cache_dir;
        self
    }

    pub fn with_python_executable(mut self, executable: String) -> Self {
        self.python_executable = executable;
        self
    }

    pub fn with_python_auto_detect(mut self) -> Self {
        let candidates = ["python", "python3"];

        for candidate in candidates.iter() {
            if std::process::Command::new(candidate)
                .arg("--version")
                .output()
                .is_ok()
            {
                self.python_executable = candidate.to_string();
                debug!("Auto-detected Python executable: {candidate}");
                break;
            }
        }

        self
    }

    /// Ensure cache directory exists
    async fn ensure_cache_dir(&self) -> Result<()> {
        if !self.cache_dir.exists() {
            fs::create_dir_all(&self.cache_dir)
                .await
                .context("Failed to create cache directory")?;
            debug!("Created cache directory: {}", self.cache_dir.display());
        }
        Ok(())
    }

    /// Send a message to the plugin using file-based communication
    pub async fn send_message(&self, message: PluginMessage) -> Result<PluginResponse> {
        debug!("Sending message to plugin: {}", self.plugin_path.display());

        self.ensure_cache_dir().await?;

        let input_filename = format!("plugin_input_{}.json", Uuid::new_v4());
        let input_file_path = self.cache_dir.join(&input_filename);

        let message_json =
            serde_json::to_string_pretty(&message).context("Failed to serialize plugin message")?;

        fs::write(&input_file_path, &message_json)
            .await
            .context("Failed to write plugin input file")?;

        debug!("Wrote plugin input to: {}", input_file_path.display());

        let result = tokio::time::timeout(Duration::from_secs(30), async {
            let input_file =
                std::fs::File::open(&input_file_path).context("Failed to open input file")?;

            let child = Command::new(&self.python_executable)
                .arg(&self.plugin_path)
                .stdin(Stdio::from(input_file))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .context(format!(
                    "Failed to spawn plugin process: {} {}",
                    self.python_executable,
                    self.plugin_path.display()
                ))?;

            let output = child
                .wait_with_output()
                .await
                .context("Failed to wait for plugin process")?;

            Ok::<std::process::Output, anyhow::Error>(output)
        })
        .await;

        let _ = fs::remove_file(&input_file_path).await;

        let output = match result {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(anyhow::anyhow!("Plugin process timed out after 30 seconds"));
            }
        };

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        debug!("Plugin stdout length: {} chars", stdout_str.len());
        if !stderr_str.is_empty() {
            debug!("Plugin stderr: {stderr_str}");
        }

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Plugin exited with non-zero status: {}. Stdout: {}. Stderr: {}",
                output.status,
                stdout_str.trim(),
                stderr_str.trim()
            ));
        }

        if stdout_str.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "Plugin produced no output. Stderr: {}",
                stderr_str.trim()
            ));
        }

        let response_line = stdout_str
            .lines()
            .find(|line| !line.trim().is_empty() && line.trim().starts_with('{'))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No valid JSON response found in plugin output. Stdout: {}",
                    stdout_str.trim()
                )
            })?;

        debug!("Plugin JSON response: {response_line}");

        let response: PluginResponse = serde_json::from_str(response_line.trim()).context(
            format!("Failed to parse plugin response JSON: {response_line}"),
        )?;

        Ok(response)
    }

    /// Get plugin information with type identification
    pub async fn get_info(&self) -> Result<PluginInfo> {
        let message = PluginMessage::GetInfo;

        match self.send_message(message).await? {
            PluginResponse::Info {
                name,
                version,
                plugin_type,
                supported_extensions,
                supported_filenames,
                supported_output_types,
                supported_formats,
            } => Ok(PluginInfo {
                name,
                version,
                plugin_type,
                supported_extensions,
                supported_filenames,
                supported_output_types,
                supported_formats,
            }),
            PluginResponse::Error { message, details } => Err(anyhow::anyhow!(
                "Plugin info request failed: {} {:?}",
                message,
                details
            )),
            _ => Err(anyhow::anyhow!(
                "Plugin returned unexpected response to get_info"
            )),
        }
    }

    /// Clean up old cache files
    pub async fn cleanup_cache(&self, max_age_hours: u64) -> Result<()> {
        use std::time::{Duration, SystemTime};

        let cutoff_time = SystemTime::now() - Duration::from_secs(max_age_hours * 3600);

        let mut dir_entries = fs::read_dir(&self.cache_dir)
            .await
            .context("Failed to read cache directory")?;

        while let Some(entry) = dir_entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        if modified < cutoff_time {
                            if let Err(e) = fs::remove_file(&path).await {
                                warn!("Failed to remove old cache file {}: {}", path.display(), e);
                            } else {
                                debug!("Removed old cache file: {}", path.display());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl PluginInterface for PluginCommunicator {
    async fn get_info(&self) -> Result<PluginInfo> {
        self.get_info().await
    }

    async fn get_plugin_type(&self) -> Result<PluginType> {
        let info = self.get_info().await?;
        Ok(info.plugin_type)
    }
}

/// Specialized communicator for input plugins (code analyzers)
pub struct InputPluginCommunicator {
    base: PluginCommunicator,
}

impl InputPluginCommunicator {
    pub fn new(plugin_path: PathBuf) -> Self {
        Self {
            base: PluginCommunicator::new(plugin_path),
        }
    }

    pub fn with_cache_dir(mut self, cache_dir: PathBuf) -> Self {
        self.base = self.base.with_cache_dir(cache_dir);
        self
    }

    pub fn with_python_executable(mut self, executable: String) -> Self {
        self.base = self.base.with_python_executable(executable);
        self
    }

    pub fn with_python_auto_detect(mut self) -> Self {
        self.base = self.base.with_python_auto_detect();
        self
    }
}

#[async_trait::async_trait]
impl PluginInterface for InputPluginCommunicator {
    async fn get_info(&self) -> Result<PluginInfo> {
        self.base.get_info().await
    }

    async fn get_plugin_type(&self) -> Result<PluginType> {
        self.base.get_plugin_type().await
    }
}

#[async_trait::async_trait]
impl InputPluginInterface for InputPluginCommunicator {
    async fn can_analyze(&self, file_path: &Path, content_preview: &str) -> Result<bool> {
        let message = PluginMessage::CanAnalyze {
            file_path: file_path.to_path_buf(),
            content_preview: content_preview.chars().take(500).collect(),
        };

        match self.base.send_message(message).await? {
            PluginResponse::CanAnalyze {
                can_analyze,
                confidence: _,
            } => Ok(can_analyze),
            PluginResponse::Error { message, details } => {
                error!("Plugin error during can_analyze: {message} {details:?}");
                Ok(false)
            }
            _ => {
                warn!("Plugin returned unexpected response to can_analyze");
                Ok(false)
            }
        }
    }

    async fn analyze(&self, input: PluginInput) -> Result<crate::plugins::interface::PluginOutput> {
        let message = PluginMessage::Analyze { input };

        match self.base.send_message(message).await? {
            PluginResponse::Success {
                cache_file,
                processing_time_ms,
            } => {
                let cache_file_path = self.base.cache_dir.join(&cache_file);

                debug!(
                    "Reading analysis result from cache file: {}",
                    cache_file_path.display()
                );

                let cache_content = fs::read_to_string(&cache_file_path).await.context(format!(
                    "Failed to read cache file: {}",
                    cache_file_path.display()
                ))?;

                let plugin_output: crate::plugins::interface::PluginOutput =
                    serde_json::from_str(&cache_content)
                        .context("Failed to parse cached analysis result")?;

                debug!("Successfully loaded analysis result from cache, processing time: {processing_time_ms}ms");

                Ok(plugin_output)
            }
            PluginResponse::Error { message, details } => Err(anyhow::anyhow!(
                "Plugin analysis failed: {} {:?}",
                message,
                details
            )),
            _ => Err(anyhow::anyhow!(
                "Plugin returned unexpected response to analyze"
            )),
        }
    }
}

/// Specialized communicator for output plugins (documentation generators, etc.)
pub struct OutputPluginCommunicator {
    base: PluginCommunicator,
}

impl OutputPluginCommunicator {
    pub fn new(plugin_path: PathBuf) -> Self {
        Self {
            base: PluginCommunicator::new(plugin_path),
        }
    }

    pub fn with_cache_dir(mut self, cache_dir: PathBuf) -> Self {
        self.base = self.base.with_cache_dir(cache_dir);
        self
    }

    pub fn with_python_executable(mut self, executable: String) -> Self {
        self.base = self.base.with_python_executable(executable);
        self
    }

    pub fn with_python_auto_detect(mut self) -> Self {
        self.base = self.base.with_python_auto_detect();
        self
    }
}

#[async_trait::async_trait]
impl PluginInterface for OutputPluginCommunicator {
    async fn get_info(&self) -> Result<PluginInfo> {
        self.base.get_info().await
    }

    async fn get_plugin_type(&self) -> Result<PluginType> {
        self.base.get_plugin_type().await
    }
}

#[async_trait::async_trait]
impl OutputPluginInterface for OutputPluginCommunicator {
    async fn can_generate(&self, output_type: &str, format: &str) -> Result<bool> {
        let message = PluginMessage::CanGenerate {
            output_type: output_type.to_string(),
            format: format.to_string(),
        };

        match self.base.send_message(message).await? {
            PluginResponse::CanGenerate {
                can_generate,
                confidence: _,
            } => Ok(can_generate),
            PluginResponse::Error { message, details } => {
                error!("Plugin error during can_generate: {message} {details:?}");
                Ok(false)
            }
            _ => {
                warn!("Plugin returned unexpected response to can_generate");
                Ok(false)
            }
        }
    }

    async fn generate(&self, input: OutputPluginInput) -> Result<OutputPluginResult> {
        let message = PluginMessage::Generate { input };

        match self.base.send_message(message).await? {
            PluginResponse::OutputSuccess { result } => {
                debug!(
                    "Output plugin generation successful: {} outputs",
                    result.outputs.len()
                );
                Ok(result)
            }
            PluginResponse::Error { message, details } => Err(anyhow::anyhow!(
                "Plugin generation failed: {} {:?}",
                message,
                details
            )),
            _ => Err(anyhow::anyhow!(
                "Plugin returned unexpected response to generate"
            )),
        }
    }

    async fn get_supported_output_types(&self) -> Result<Vec<String>> {
        let info = self.base.get_info().await?;
        Ok(info.supported_output_types.unwrap_or_default())
    }

    async fn get_supported_formats(&self) -> Result<Vec<String>> {
        let info = self.base.get_info().await?;
        Ok(info.supported_formats.unwrap_or_default())
    }
}

// Legacy compatibility - maintain the original PluginCommunicator for existing code
impl PluginCommunicator {
    /// Legacy method for backward compatibility
    pub async fn can_analyze(&self, file_path: &Path, content_preview: &str) -> Result<bool> {
        let input_comm = InputPluginCommunicator::new(self.plugin_path.clone())
            .with_cache_dir(self.cache_dir.clone())
            .with_python_executable(self.python_executable.clone());

        input_comm.can_analyze(file_path, content_preview).await
    }

    /// Legacy method for backward compatibility
    pub async fn analyze(
        &self,
        input: PluginInput,
    ) -> Result<crate::plugins::interface::PluginOutput> {
        let input_comm = InputPluginCommunicator::new(self.plugin_path.clone())
            .with_cache_dir(self.cache_dir.clone())
            .with_python_executable(self.python_executable.clone());

        input_comm.analyze(input).await
    }
}
