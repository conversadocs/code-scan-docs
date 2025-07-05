use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    Input,
    Output,
}

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

    pub token_info: Option<serde_json::Value>,

    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPluginResult {
    pub plugin_name: String,
    pub plugin_version: String,
    pub output_type: String,
    pub outputs: Vec<GeneratedOutput>,
    pub processing_time_ms: u64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedOutput {
    pub output_path: PathBuf,
    pub content_type: String,
    pub size_bytes: u64,
    pub checksum: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeElement {
    pub element_type: String,
    pub name: String,
    pub signature: Option<String>,
    pub line_start: u32,
    pub line_end: u32,
    pub summary: Option<String>,
    pub complexity_score: Option<u32>,
    pub calls: Vec<String>,
    pub metadata: serde_json::Value,

    pub tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub module: String,
    pub items: Vec<String>,
    pub alias: Option<String>,
    pub line_number: u32,
    pub import_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from_file: String,
    pub to_file: String,
    pub relationship_type: String,
    pub details: String,
    pub line_number: Option<u32>,
    pub strength: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDependency {
    pub name: String,
    pub version: Option<String>,
    pub ecosystem: String,
    pub dependency_type: String,
    pub source_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInput {
    pub file_path: PathBuf,
    pub relative_path: PathBuf,
    pub content: String,
    pub project_root: PathBuf,
    pub cache_dir: String,
    pub plugin_config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPluginInput {
    pub matrix_path: PathBuf,
    pub project_root: PathBuf,
    pub output_dir: PathBuf,
    pub cache_dir: String,
    pub plugin_config: Option<serde_json::Value>,
    pub format_options: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PluginMessage {
    #[serde(rename = "analyze")]
    Analyze { input: PluginInput },

    #[serde(rename = "generate")]
    Generate { input: OutputPluginInput },

    #[serde(rename = "can_analyze")]
    CanAnalyze {
        file_path: PathBuf,
        content_preview: String,
    },

    #[serde(rename = "can_generate")]
    CanGenerate { output_type: String, format: String },

    #[serde(rename = "get_info")]
    GetInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum PluginResponse {
    #[serde(rename = "success")]
    Success {
        cache_file: String,
        processing_time_ms: u64,
    },

    #[serde(rename = "output_success")]
    OutputSuccess { result: OutputPluginResult },

    #[serde(rename = "can_analyze")]
    CanAnalyze { can_analyze: bool, confidence: f32 },

    #[serde(rename = "can_generate")]
    CanGenerate { can_generate: bool, confidence: f32 },

    #[serde(rename = "info")]
    Info {
        name: String,
        version: String,
        plugin_type: PluginType,
        supported_extensions: Vec<String>,
        supported_filenames: Vec<String>,
        supported_output_types: Option<Vec<String>>,
        supported_formats: Option<Vec<String>>,
    },

    #[serde(rename = "error")]
    Error {
        message: String,
        details: Option<String>,
    },
}

#[async_trait::async_trait]
pub trait PluginInterface {
    async fn get_info(&self) -> anyhow::Result<PluginInfo>;
    async fn get_plugin_type(&self) -> anyhow::Result<PluginType>;
}

#[async_trait::async_trait]
pub trait InputPluginInterface: PluginInterface {
    async fn can_analyze(&self, file_path: &Path, content_preview: &str) -> anyhow::Result<bool>;
    async fn analyze(&self, input: PluginInput) -> anyhow::Result<PluginOutput>;
}

#[async_trait::async_trait]
pub trait OutputPluginInterface: PluginInterface {
    async fn can_generate(&self, output_type: &str, format: &str) -> anyhow::Result<bool>;
    async fn generate(&self, input: OutputPluginInput) -> anyhow::Result<OutputPluginResult>;
    async fn get_supported_output_types(&self) -> anyhow::Result<Vec<String>>;
    async fn get_supported_formats(&self) -> anyhow::Result<Vec<String>>;
}

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub plugin_type: PluginType,
    pub supported_extensions: Vec<String>,
    pub supported_filenames: Vec<String>,
    pub supported_output_types: Option<Vec<String>>,
    pub supported_formats: Option<Vec<String>>,
}

impl PluginInfo {
    pub fn is_input_plugin(&self) -> bool {
        self.plugin_type == PluginType::Input
    }

    pub fn is_output_plugin(&self) -> bool {
        self.plugin_type == PluginType::Output
    }

    pub fn get_capabilities_description(&self) -> String {
        match self.plugin_type {
            PluginType::Input => {
                let mut caps = Vec::new();
                if !self.supported_extensions.is_empty() {
                    caps.push(format!(
                        "Extensions: {}",
                        self.supported_extensions.join(", ")
                    ));
                }
                if !self.supported_filenames.is_empty() {
                    caps.push(format!("Files: {}", self.supported_filenames.join(", ")));
                }
                caps.join(" | ")
            }
            PluginType::Output => {
                let mut caps = Vec::new();
                if let Some(ref types) = self.supported_output_types {
                    caps.push(format!("Types: {}", types.join(", ")));
                }
                if let Some(ref formats) = self.supported_formats {
                    caps.push(format!("Formats: {}", formats.join(", ")));
                }
                caps.join(" | ")
            }
        }
    }
}

pub struct PluginFactory;

impl PluginFactory {
    pub async fn detect_plugin_type(plugin_path: &Path) -> anyhow::Result<PluginType> {
        use crate::plugins::communication::PluginCommunicator;

        let communicator = PluginCommunicator::new(plugin_path.to_path_buf());
        let info = communicator.get_info().await?;
        Ok(info.plugin_type)
    }

    pub async fn create_plugin_communicator(
        plugin_path: PathBuf,
    ) -> anyhow::Result<Box<dyn PluginInterface + Send + Sync>> {
        let plugin_type = Self::detect_plugin_type(&plugin_path).await?;

        match plugin_type {
            PluginType::Input => Ok(Box::new(
                crate::plugins::communication::InputPluginCommunicator::new(plugin_path),
            )),
            PluginType::Output => Ok(Box::new(
                crate::plugins::communication::OutputPluginCommunicator::new(plugin_path),
            )),
        }
    }
}
