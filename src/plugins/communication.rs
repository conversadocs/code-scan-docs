use anyhow::{Result, Context};
use log::{debug, warn, error};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use serde_json;

use crate::plugins::interface::{PluginInput, PluginResponse, PluginMessage};

pub struct PluginCommunicator {
    plugin_path: PathBuf,
    python_executable: String,
}

impl PluginCommunicator {
    pub fn new(plugin_path: PathBuf) -> Self {
        Self {
            plugin_path,
            python_executable: "python".to_string(), // Changed from python3 to python
        }
    }

    pub fn with_python_executable(mut self, executable: String) -> Self {
        self.python_executable = executable;
        self
    }

    /// Send a message to the plugin and get a response
    pub async fn send_message(&self, message: PluginMessage) -> Result<PluginResponse> {
        debug!("Sending message to plugin: {}", self.plugin_path.display());

        // Serialize the message
        let message_json = serde_json::to_string(&message)
            .context("Failed to serialize plugin message")?;

        debug!("Plugin message: {}", message_json);

        // Spawn the plugin process
        let mut child = Command::new(&self.python_executable)
            .arg(&self.plugin_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn plugin process")?;

        // Get handles to stdin/stdout
        let stdin = child.stdin.as_mut()
            .context("Failed to get stdin handle")?;
        let stdout = child.stdout.take()
            .context("Failed to get stdout handle")?;
        let stderr = child.stderr.take()
            .context("Failed to get stderr handle")?;

        // Send the message
        stdin.write_all(message_json.as_bytes()).await
            .context("Failed to write to plugin stdin")?;
        stdin.write_all(b"\n").await
            .context("Failed to write newline to plugin stdin")?;
        stdin.shutdown().await
            .context("Failed to close plugin stdin")?;

        // Read the response
        let mut stdout_reader = BufReader::new(stdout);
        let mut stderr_reader = BufReader::new(stderr);

        let mut response_line = String::new();
        let mut stderr_output = String::new();

        // Read response from stdout
        match stdout_reader.read_line(&mut response_line).await {
            Ok(0) => {
                // No output, check stderr
                let _ = stderr_reader.read_to_string(&mut stderr_output).await;
                return Err(anyhow::anyhow!(
                    "Plugin produced no output. Stderr: {}",
                    stderr_output
                ));
            }
            Ok(_) => {
                debug!("Plugin response: {}", response_line.trim());
            }
            Err(e) => {
                let _ = stderr_reader.read_to_string(&mut stderr_output).await;
                return Err(anyhow::anyhow!(
                    "Failed to read plugin response: {}. Stderr: {}",
                    e, stderr_output
                ));
            }
        }

        // Also capture any stderr for debugging
        let _ = stderr_reader.read_to_string(&mut stderr_output).await;
        if !stderr_output.is_empty() {
            warn!("Plugin stderr: {}", stderr_output);
        }

        // Wait for the process to complete
        let exit_status = child.wait().await
            .context("Failed to wait for plugin process")?;

        if !exit_status.success() {
            return Err(anyhow::anyhow!(
                "Plugin exited with non-zero status: {}. Stderr: {}",
                exit_status, stderr_output
            ));
        }

        // Parse the response
        let response: PluginResponse = serde_json::from_str(response_line.trim())
            .context("Failed to parse plugin response JSON")?;

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

    /// Analyze a file with this plugin
    pub async fn analyze(&self, input: PluginInput) -> Result<crate::plugins::interface::PluginOutput> {
        let message = PluginMessage::Analyze { input };

        match self.send_message(message).await? {
            PluginResponse::Success { data } => Ok(data),
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
}
