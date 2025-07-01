use anyhow::{Result, Context};
use log::{debug, warn, error, info};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::fs;
use serde_json;
use std::time::Duration;
use uuid::Uuid;

use crate::plugins::interface::{PluginInput, PluginResponse, PluginMessage};

pub struct PluginCommunicator {
    plugin_path: PathBuf,
    python_executable: String,
    cache_dir: PathBuf,
}

impl PluginCommunicator {
    pub fn new(plugin_path: PathBuf) -> Self {
        // Default cache directory
        let cache_dir = PathBuf::from(".csd_cache");

        Self {
            plugin_path,
            python_executable: "python".to_string(), // Use python (works with pyenv)
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
        // Try to detect the best Python executable
        // Priority: python (pyenv), python3, python
        let candidates = ["python", "python3"];

        for candidate in candidates.iter() {
            if std::process::Command::new(candidate)
                .arg("--version")
                .output()
                .is_ok()
            {
                self.python_executable = candidate.to_string();
                debug!("Auto-detected Python executable: {}", candidate);
                break;
            }
        }

        self
    }

    /// Ensure cache directory exists
    async fn ensure_cache_dir(&self) -> Result<()> {
        if !self.cache_dir.exists() {
            fs::create_dir_all(&self.cache_dir).await
                .context("Failed to create cache directory")?;
            debug!("Created cache directory: {}", self.cache_dir.display());
        }
        Ok(())
    }

    /// Send a message to the plugin using file-based communication
    pub async fn send_message(&self, message: PluginMessage) -> Result<PluginResponse> {
        debug!("Sending message to plugin: {}", self.plugin_path.display());

        // Ensure cache directory exists
        self.ensure_cache_dir().await?;

        // Create temporary input file
        let input_filename = format!("plugin_input_{}.json", Uuid::new_v4().to_string());
        let input_file_path = self.cache_dir.join(&input_filename);

        // Write message to temporary file
        let message_json = serde_json::to_string_pretty(&message)
            .context("Failed to serialize plugin message")?;

        fs::write(&input_file_path, &message_json).await
            .context("Failed to write plugin input file")?;

        debug!("Wrote plugin input to: {}", input_file_path.display());

        // Run the plugin with the input file
        let result = tokio::time::timeout(Duration::from_secs(30), async {
            let input_file = std::fs::File::open(&input_file_path)
                .context("Failed to open input file")?;

            let mut child = Command::new(&self.python_executable)
                .arg(&self.plugin_path)
                .stdin(Stdio::from(input_file))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .context(format!("Failed to spawn plugin process: {} {}",
                    self.python_executable, self.plugin_path.display()))?;

            // Read stdout and stderr
            let output = child.wait_with_output().await
                .context("Failed to wait for plugin process")?;

            Ok::<std::process::Output, anyhow::Error>(output)
        }).await;

        // Clean up input file
        let _ = fs::remove_file(&input_file_path).await;

        let output = match result {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                return Err(anyhow::anyhow!("Plugin process timed out after 30 seconds"));
            }
        };

        // Convert to strings
        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);

        debug!("Plugin stdout length: {} chars", stdout_str.len());
        if !stderr_str.is_empty() {
            debug!("Plugin stderr: {}", stderr_str);
        }

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Plugin exited with non-zero status: {}. Stdout: {}. Stderr: {}",
                output.status, stdout_str.trim(), stderr_str.trim()
            ));
        }

        if stdout_str.trim().is_empty() {
            return Err(anyhow::anyhow!(
                "Plugin produced no output. Stderr: {}",
                stderr_str.trim()
            ));
        }

        // Parse the response - it should be a single JSON line
        let response_line = stdout_str.lines()
            .find(|line| !line.trim().is_empty() && line.trim().starts_with('{'))
            .ok_or_else(|| anyhow::anyhow!(
                "No valid JSON response found in plugin output. Stdout: {}",
                stdout_str.trim()
            ))?;

        debug!("Plugin JSON response: {}", response_line);

        // Parse the response
        let response: PluginResponse = serde_json::from_str(response_line.trim())
            .context(format!("Failed to parse plugin response JSON: {}", response_line))?;

        Ok(response)
    }

    /// Check if this plugin can analyze the given file
    pub async fn can_analyze(&self, file_path: &Path, content_preview: &str) -> Result<bool> {
        let message = PluginMessage::CanAnalyze {
            file_path: file_path.to_path_buf(),
            content_preview: content_preview.chars().take(500).collect(), // First 500 chars
        };

        match self.send_message(message).await? {
            PluginResponse::CanAnalyze { can_analyze, confidence: _ } => Ok(can_analyze),
            PluginResponse::Error { message, details } => {
                error!("Plugin error during can_analyze: {} {:?}", message, details);
                Ok(false) // If plugin errors, assume it can't analyze
            }
            _ => {
                warn!("Plugin returned unexpected response to can_analyze");
                Ok(false)
            }
        }
    }

    /// Analyze a file with this plugin using file-based communication
    pub async fn analyze(&self, input: PluginInput) -> Result<crate::plugins::interface::PluginOutput> {
        let message = PluginMessage::Analyze { input };

        match self.send_message(message).await? {
            PluginResponse::Success { cache_file, processing_time_ms } => {
                // Read the analysis result from the cache file
                let cache_file_path = self.cache_dir.join(&cache_file);

                debug!("Reading analysis result from cache file: {}", cache_file_path.display());

                let cache_content = fs::read_to_string(&cache_file_path).await
                    .context(format!("Failed to read cache file: {}", cache_file_path.display()))?;

                let plugin_output: crate::plugins::interface::PluginOutput =
                    serde_json::from_str(&cache_content)
                        .context("Failed to parse cached analysis result")?;

                debug!("Successfully loaded analysis result from cache, processing time: {}ms", processing_time_ms);

                Ok(plugin_output)
            }
            PluginResponse::Error { message, details } => {
                Err(anyhow::anyhow!("Plugin analysis failed: {} {:?}", message, details))
            }
            _ => {
                Err(anyhow::anyhow!("Plugin returned unexpected response to analyze"))
            }
        }
    }

    /// Get plugin information
    pub async fn get_info(&self) -> Result<crate::plugins::interface::PluginInfo> {
        let message = PluginMessage::GetInfo;

        match self.send_message(message).await? {
            PluginResponse::Info { name, version, supported_extensions, supported_filenames } => {
                Ok(crate::plugins::interface::PluginInfo {
                    name,
                    version,
                    supported_extensions,
                    supported_filenames,
                })
            }
            PluginResponse::Error { message, details } => {
                Err(anyhow::anyhow!("Plugin info request failed: {} {:?}", message, details))
            }
            _ => {
                Err(anyhow::anyhow!("Plugin returned unexpected response to get_info"))
            }
        }
    }

    /// Clean up old cache files (optional utility method)
    pub async fn cleanup_cache(&self, max_age_hours: u64) -> Result<()> {
        use std::time::{SystemTime, Duration};

        let cutoff_time = SystemTime::now() - Duration::from_secs(max_age_hours * 3600);

        let mut dir_entries = fs::read_dir(&self.cache_dir).await
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
