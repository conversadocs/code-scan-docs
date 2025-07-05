//! Plugin interface definitions and data structures.
//!
//! This module defines the core interfaces, data structures, and communication
//! protocols for both input (analysis) and output (generation) plugins. It provides
//! the contract that all plugins must implement and the data types used for
//! plugin communication.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Type classification for plugins.
///
/// Distinguishes between plugins that analyze code (input) and plugins
/// that generate output (output).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    Input,
    Output,
}

/// Complete analysis result from an input plugin.
///
/// Contains all information extracted from a source file including code elements,
/// dependencies, relationships, and metadata. This is the primary data structure
/// returned by input plugins after analyzing a file.
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

/// Result of an output plugin generation operation.
///
/// Contains information about generated files, processing metadata, and
/// plugin details. Returned by output plugins after successful generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPluginResult {
    pub plugin_name: String,
    pub plugin_version: String,
    pub output_type: String,
    pub outputs: Vec<GeneratedOutput>,
    pub processing_time_ms: u64,
    pub metadata: serde_json::Value,
}

/// Information about a single generated output file.
///
/// Describes a file created by an output plugin including its location,
/// type, size, and verification checksum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedOutput {
    pub output_path: PathBuf,
    pub content_type: String,
    pub size_bytes: u64,
    pub checksum: String,
    pub metadata: serde_json::Value,
}

/// A code element discovered within a source file.
///
/// Represents functions, classes, variables, or other code constructs
/// identified by input plugins during analysis.
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

/// An import or dependency statement found in source code.
///
/// Represents how a file imports functionality from other modules,
/// libraries, or files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub module: String,
    pub items: Vec<String>,
    pub alias: Option<String>,
    pub line_number: u32,
    pub import_type: String,
}

/// A relationship between files discovered during analysis.
///
/// Represents connections like imports, function calls, inheritance,
/// or other dependencies between source files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from_file: String,
    pub to_file: String,
    pub relationship_type: String,
    pub details: String,
    pub line_number: Option<u32>,
    pub strength: f32,
}

/// An external library or framework dependency.
///
/// Represents dependencies on code outside the project, such as
/// third-party libraries, standard library modules, or frameworks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDependency {
    pub name: String,
    pub version: Option<String>,
    pub ecosystem: String,
    pub dependency_type: String,
    pub source_file: String,
}

/// Input data provided to input plugins for analysis.
///
/// Contains the file content, metadata, and configuration needed
/// for plugins to analyze source files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInput {
    pub file_path: PathBuf,
    pub relative_path: PathBuf,
    pub content: String,
    pub project_root: PathBuf,
    pub cache_dir: String,
    pub plugin_config: Option<serde_json::Value>,
}

/// Input data provided to output plugins for generation.
///
/// Contains the project matrix, output configuration, and settings
/// needed for plugins to generate documentation or reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputPluginInput {
    pub matrix_path: PathBuf,
    pub project_root: PathBuf,
    pub output_dir: PathBuf,
    pub cache_dir: String,
    pub plugin_config: Option<serde_json::Value>,
    pub format_options: serde_json::Value,
}

/// Messages that can be sent to plugins.
///
/// Defines the communication protocol between the application and
/// plugins, including analysis requests, generation commands, and
/// capability queries.
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

/// Responses that plugins can send back to the application.
///
/// Defines the possible response types from plugins including
/// success results, capability confirmations, and error messages.
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

/// Base interface that all plugins must implement.
///
/// Provides common functionality for querying plugin metadata
/// and capabilities.
#[async_trait::async_trait]
pub trait PluginInterface {
    async fn get_info(&self) -> anyhow::Result<PluginInfo>;
    async fn get_plugin_type(&self) -> anyhow::Result<PluginType>;
}

/// Interface for input (analysis) plugins.
///
/// Extends the base plugin interface with methods for analyzing
/// source files and determining file compatibility.
#[async_trait::async_trait]
pub trait InputPluginInterface: PluginInterface {
    async fn can_analyze(&self, file_path: &Path, content_preview: &str) -> anyhow::Result<bool>;
    async fn analyze(&self, input: PluginInput) -> anyhow::Result<PluginOutput>;
}

/// Interface for output (generation) plugins.
///
/// Extends the base plugin interface with methods for generating
/// documentation, reports, and other output formats.
#[async_trait::async_trait]
pub trait OutputPluginInterface: PluginInterface {
    async fn can_generate(&self, output_type: &str, format: &str) -> anyhow::Result<bool>;
    async fn generate(&self, input: OutputPluginInput) -> anyhow::Result<OutputPluginResult>;
    async fn get_supported_output_types(&self) -> anyhow::Result<Vec<String>>;
    async fn get_supported_formats(&self) -> anyhow::Result<Vec<String>>;
}

/// Metadata and capability information for a plugin.
///
/// Contains the plugin's name, version, type, and supported file types
/// or output formats. Used for plugin discovery and selection.
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
    /// Checks if this is an input (analysis) plugin.
    ///
    /// # Returns
    ///
    /// `true` if this plugin analyzes input files, `false` otherwise.
    pub fn is_input_plugin(&self) -> bool {
        self.plugin_type == PluginType::Input
    }

    /// Checks if this is an output (generation) plugin.
    ///
    /// # Returns
    ///
    /// `true` if this plugin generates output, `false` otherwise.
    pub fn is_output_plugin(&self) -> bool {
        self.plugin_type == PluginType::Output
    }

    /// Returns a human-readable description of the plugin's capabilities.
    ///
    /// Formats the supported extensions, filenames, output types, or formats
    /// into a readable string for display purposes.
    ///
    /// # Returns
    ///
    /// A formatted string describing what the plugin can handle.
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

/// Factory for creating and detecting plugin types.
///
/// Provides utilities for automatically detecting plugin types and
/// creating appropriate communicator instances.
pub struct PluginFactory;

impl PluginFactory {
    /// Automatically detects the type of a plugin by querying it.
    ///
    /// Communicates with the plugin to determine whether it's an input
    /// or output plugin.
    ///
    /// # Arguments
    ///
    /// * `plugin_path` - Path to the plugin to analyze
    ///
    /// # Returns
    ///
    /// The detected plugin type or an error if detection fails.
    pub async fn detect_plugin_type(plugin_path: &Path) -> anyhow::Result<PluginType> {
        use crate::plugins::communication::PluginCommunicator;

        let communicator = PluginCommunicator::new(plugin_path.to_path_buf());
        let info = communicator.get_info().await?;
        Ok(info.plugin_type)
    }

    /// Creates an appropriate communicator for the given plugin.
    ///
    /// Detects the plugin type and returns a type-specific communicator
    /// instance ready for use.
    ///
    /// # Arguments
    ///
    /// * `plugin_path` - Path to the plugin
    ///
    /// # Returns
    ///
    /// A boxed plugin interface or an error if creation fails.
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
