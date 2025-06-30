use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use crate::core::matrix::{CodeElement, Import, Relationship, ExternalDependency};

/// Standard output format that all plugins must produce
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginOutput {
    pub file_path: PathBuf,
    pub file_hash: String,
    pub elements: Vec<CodeElement>,
    pub imports: Vec<Import>,
    pub exports: Vec<String>,
    pub relationships: Vec<Relationship>,
    pub external_dependencies: Vec<ExternalDependency>,
    pub file_summary: Option<String>,
    pub processing_time_ms: u64,
    pub plugin_version: String,
}

/// Input sent to plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInput {
    pub file_path: PathBuf,
    pub relative_path: PathBuf,
    pub content: String,
    pub project_root: PathBuf,
    pub plugin_config: Option<serde_json::Value>,
}

/// Plugin communication protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PluginMessage {
    #[serde(rename = "analyze")]
    Analyze { input: PluginInput },

    #[serde(rename = "can_analyze")]
    CanAnalyze { file_path: PathBuf, content_preview: String },

    #[serde(rename = "get_info")]
    GetInfo,
}

/// Plugin response protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum PluginResponse {
    #[serde(rename = "success")]
    Success { data: PluginOutput },

    #[serde(rename = "can_analyze")]
    CanAnalyze { can_analyze: bool, confidence: f32 },

    #[serde(rename = "info")]
    Info {
        name: String,
        version: String,
        supported_extensions: Vec<String>,
        supported_filenames: Vec<String>,
    },

    #[serde(rename = "error")]
    Error { message: String, details: Option<String> },
}

/// Trait for implementing plugin communication
#[async_trait::async_trait]
pub trait PluginInterface {
    async fn can_analyze(&self, file_path: &PathBuf, content_preview: &str) -> anyhow::Result<bool>;
    async fn analyze(&self, input: PluginInput) -> anyhow::Result<PluginOutput>;
    async fn get_info(&self) -> anyhow::Result<PluginInfo>;
}

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub supported_extensions: Vec<String>,
    pub supported_filenames: Vec<String>,
}
