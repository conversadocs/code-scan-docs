use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

/// Code element structure for plugin communication (uses strings, not enums)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeElement {
    pub element_type: String, // "function", "class", "method", etc.
    pub name: String,
    pub signature: Option<String>,
    pub line_start: u32,
    pub line_end: u32,
    pub summary: Option<String>,
    pub complexity_score: Option<u32>,
    pub calls: Vec<String>,
    pub metadata: serde_json::Value,
}

/// Import structure for plugin communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub module: String,
    pub items: Vec<String>,
    pub alias: Option<String>,
    pub line_number: u32,
    pub import_type: String, // "standard", "third_party", "local", "relative"
}

/// Relationship structure for plugin communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from_file: String,
    pub to_file: String,
    pub relationship_type: String, // "import", "call", "inheritance", etc.
    pub details: String,
    pub line_number: Option<u32>,
    pub strength: f32,
}

/// External dependency structure for plugin communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDependency {
    pub name: String,
    pub version: Option<String>,
    pub ecosystem: String,       // "pip", "npm", "cargo", etc.
    pub dependency_type: String, // "runtime", "development", "build", "optional"
    pub source_file: String,
}

/// Input sent to plugins - now includes cache_dir
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInput {
    pub file_path: PathBuf,
    pub relative_path: PathBuf,
    pub content: String,
    pub project_root: PathBuf,
    pub cache_dir: String, // Added this field
    pub plugin_config: Option<serde_json::Value>,
}

/// Plugin communication protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PluginMessage {
    #[serde(rename = "analyze")]
    Analyze { input: PluginInput },

    #[serde(rename = "can_analyze")]
    CanAnalyze {
        file_path: PathBuf,
        content_preview: String,
    },

    #[serde(rename = "get_info")]
    GetInfo,
}

/// Plugin response protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum PluginResponse {
    #[serde(rename = "success")]
    Success {
        cache_file: String,
        processing_time_ms: u64, // Changed from i32 to u64 for consistency
    },

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
    Error {
        message: String,
        details: Option<String>,
    },
}

/// Trait for implementing plugin communication
#[async_trait::async_trait]
pub trait PluginInterface {
    async fn can_analyze(&self, file_path: &Path, content_preview: &str) -> anyhow::Result<bool>;
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

// Unit tests

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use std::path::PathBuf;

    // Helper function to create a test CodeElement
    fn create_test_code_element() -> CodeElement {
        CodeElement {
            element_type: "function".to_string(),
            name: "test_function".to_string(),
            signature: Some("fn test_function(x: i32) -> bool".to_string()),
            line_start: 10,
            line_end: 20,
            summary: Some("A test function".to_string()),
            complexity_score: Some(5),
            calls: vec![
                "helper_function".to_string(),
                "another_function".to_string(),
            ],
            metadata: serde_json::json!({
                "is_async": false,
                "visibility": "public",
                "parameters": ["x"]
            }),
        }
    }

    // Helper function to create a test Import
    fn create_test_import() -> Import {
        Import {
            module: "std::collections".to_string(),
            items: vec!["HashMap".to_string(), "HashSet".to_string()],
            alias: Some("collections".to_string()),
            line_number: 5,
            import_type: "standard".to_string(),
        }
    }

    // Helper function to create a test Relationship
    fn create_test_relationship() -> Relationship {
        Relationship {
            from_file: "src/main.rs".to_string(),
            to_file: "src/lib.rs".to_string(),
            relationship_type: "import".to_string(),
            details: "imports lib module".to_string(),
            line_number: Some(15),
            strength: 0.8,
        }
    }

    // Helper function to create a test ExternalDependency
    fn create_test_external_dependency() -> ExternalDependency {
        ExternalDependency {
            name: "serde".to_string(),
            version: Some("1.0.0".to_string()),
            ecosystem: "cargo".to_string(),
            dependency_type: "runtime".to_string(),
            source_file: "Cargo.toml".to_string(),
        }
    }

    // Helper function to create a complete test PluginOutput
    fn create_test_plugin_output() -> PluginOutput {
        PluginOutput {
            file_path: PathBuf::from("/project/src/main.rs"),
            file_hash: "abc123def456".to_string(),
            elements: vec![create_test_code_element()],
            imports: vec![create_test_import()],
            exports: vec!["main_function".to_string(), "helper".to_string()],
            relationships: vec![create_test_relationship()],
            external_dependencies: vec![create_test_external_dependency()],
            file_summary: Some("Main application file".to_string()),
            processing_time_ms: 150,
            plugin_version: "1.0.0".to_string(),
        }
    }

    // Helper function to create a test PluginInput
    fn create_test_plugin_input() -> PluginInput {
        PluginInput {
            file_path: PathBuf::from("/project/src/main.rs"),
            relative_path: PathBuf::from("src/main.rs"),
            content: "fn main() { println!(\"Hello\"); }".to_string(),
            project_root: PathBuf::from("/project"),
            cache_dir: "/project/.csd_cache".to_string(),
            plugin_config: Some(serde_json::json!({
                "analyze_comments": true,
                "max_complexity": 10
            })),
        }
    }

    #[test]
    fn test_code_element_creation() {
        let element = create_test_code_element();

        assert_eq!(element.element_type, "function");
        assert_eq!(element.name, "test_function");
        assert_eq!(element.line_start, 10);
        assert_eq!(element.line_end, 20);
        assert_eq!(element.complexity_score, Some(5));
        assert_eq!(element.calls.len(), 2);
        assert!(element.calls.contains(&"helper_function".to_string()));
        assert!(element.metadata.is_object());
    }

    #[test]
    fn test_code_element_serialization() {
        let element = create_test_code_element();

        // Serialize to JSON
        let json = serde_json::to_string(&element).expect("Failed to serialize CodeElement");
        assert!(json.contains("test_function"));
        assert!(json.contains("function"));

        // Deserialize back
        let deserialized: CodeElement =
            serde_json::from_str(&json).expect("Failed to deserialize CodeElement");

        assert_eq!(deserialized.name, element.name);
        assert_eq!(deserialized.element_type, element.element_type);
        assert_eq!(deserialized.line_start, element.line_start);
        assert_eq!(deserialized.calls, element.calls);
    }

    #[test]
    fn test_import_creation() {
        let import = create_test_import();

        assert_eq!(import.module, "std::collections");
        assert_eq!(import.items.len(), 2);
        assert!(import.items.contains(&"HashMap".to_string()));
        assert!(import.items.contains(&"HashSet".to_string()));
        assert_eq!(import.alias, Some("collections".to_string()));
        assert_eq!(import.line_number, 5);
        assert_eq!(import.import_type, "standard");
    }

    #[test]
    fn test_import_serialization() {
        let import = create_test_import();

        let json = serde_json::to_string(&import).expect("Failed to serialize Import");
        let deserialized: Import =
            serde_json::from_str(&json).expect("Failed to deserialize Import");

        assert_eq!(deserialized.module, import.module);
        assert_eq!(deserialized.items, import.items);
        assert_eq!(deserialized.alias, import.alias);
        assert_eq!(deserialized.import_type, import.import_type);
    }

    #[test]
    fn test_relationship_creation() {
        let relationship = create_test_relationship();

        assert_eq!(relationship.from_file, "src/main.rs");
        assert_eq!(relationship.to_file, "src/lib.rs");
        assert_eq!(relationship.relationship_type, "import");
        assert_eq!(relationship.strength, 0.8);
        assert_eq!(relationship.line_number, Some(15));
    }

    #[test]
    fn test_relationship_serialization() {
        let relationship = create_test_relationship();

        let json = serde_json::to_string(&relationship).expect("Failed to serialize Relationship");
        let deserialized: Relationship =
            serde_json::from_str(&json).expect("Failed to deserialize Relationship");

        assert_eq!(deserialized.from_file, relationship.from_file);
        assert_eq!(deserialized.to_file, relationship.to_file);
        assert_eq!(
            deserialized.relationship_type,
            relationship.relationship_type
        );
        assert_eq!(deserialized.strength, relationship.strength);
    }

    #[test]
    fn test_external_dependency_creation() {
        let dependency = create_test_external_dependency();

        assert_eq!(dependency.name, "serde");
        assert_eq!(dependency.version, Some("1.0.0".to_string()));
        assert_eq!(dependency.ecosystem, "cargo");
        assert_eq!(dependency.dependency_type, "runtime");
        assert_eq!(dependency.source_file, "Cargo.toml");
    }

    #[test]
    fn test_external_dependency_serialization() {
        let dependency = create_test_external_dependency();

        let json =
            serde_json::to_string(&dependency).expect("Failed to serialize ExternalDependency");
        let deserialized: ExternalDependency =
            serde_json::from_str(&json).expect("Failed to deserialize ExternalDependency");

        assert_eq!(deserialized.name, dependency.name);
        assert_eq!(deserialized.version, dependency.version);
        assert_eq!(deserialized.ecosystem, dependency.ecosystem);
        assert_eq!(deserialized.dependency_type, dependency.dependency_type);
    }

    #[test]
    fn test_plugin_input_creation() {
        let input = create_test_plugin_input();

        assert_eq!(input.file_path, PathBuf::from("/project/src/main.rs"));
        assert_eq!(input.relative_path, PathBuf::from("src/main.rs"));
        assert_eq!(input.project_root, PathBuf::from("/project"));
        assert_eq!(input.cache_dir, "/project/.csd_cache");
        assert!(input.content.contains("main"));
        assert!(input.plugin_config.is_some());
    }

    #[test]
    fn test_plugin_input_serialization() {
        let input = create_test_plugin_input();

        let json = serde_json::to_string(&input).expect("Failed to serialize PluginInput");
        let deserialized: PluginInput =
            serde_json::from_str(&json).expect("Failed to deserialize PluginInput");

        assert_eq!(deserialized.file_path, input.file_path);
        assert_eq!(deserialized.relative_path, input.relative_path);
        assert_eq!(deserialized.content, input.content);
        assert_eq!(deserialized.cache_dir, input.cache_dir);
    }

    #[test]
    fn test_plugin_output_creation() {
        let output = create_test_plugin_output();

        assert_eq!(output.file_path, PathBuf::from("/project/src/main.rs"));
        assert_eq!(output.file_hash, "abc123def456");
        assert_eq!(output.elements.len(), 1);
        assert_eq!(output.imports.len(), 1);
        assert_eq!(output.exports.len(), 2);
        assert_eq!(output.relationships.len(), 1);
        assert_eq!(output.external_dependencies.len(), 1);
        assert_eq!(output.processing_time_ms, 150);
        assert_eq!(output.plugin_version, "1.0.0");
    }

    #[test]
    fn test_plugin_output_serialization() {
        let output = create_test_plugin_output();

        let json = serde_json::to_string(&output).expect("Failed to serialize PluginOutput");
        let deserialized: PluginOutput =
            serde_json::from_str(&json).expect("Failed to deserialize PluginOutput");

        assert_eq!(deserialized.file_path, output.file_path);
        assert_eq!(deserialized.file_hash, output.file_hash);
        assert_eq!(deserialized.elements.len(), output.elements.len());
        assert_eq!(deserialized.imports.len(), output.imports.len());
        assert_eq!(deserialized.exports, output.exports);
        assert_eq!(deserialized.processing_time_ms, output.processing_time_ms);
    }

    #[test]
    fn test_plugin_message_analyze_serialization() {
        let input = create_test_plugin_input();
        let message = PluginMessage::Analyze { input };

        let json =
            serde_json::to_string(&message).expect("Failed to serialize PluginMessage::Analyze");
        assert!(json.contains("\"type\":\"analyze\""));
        assert!(json.contains("main.rs"));

        let deserialized: PluginMessage =
            serde_json::from_str(&json).expect("Failed to deserialize PluginMessage::Analyze");

        match deserialized {
            PluginMessage::Analyze {
                input: deserialized_input,
            } => {
                assert_eq!(
                    deserialized_input.file_path,
                    PathBuf::from("/project/src/main.rs")
                );
                assert_eq!(deserialized_input.cache_dir, "/project/.csd_cache");
            }
            _ => panic!("Expected Analyze message"),
        }
    }

    #[test]
    fn test_plugin_message_can_analyze_serialization() {
        let message = PluginMessage::CanAnalyze {
            file_path: PathBuf::from("test.py"),
            content_preview: "print('hello')".to_string(),
        };

        let json =
            serde_json::to_string(&message).expect("Failed to serialize PluginMessage::CanAnalyze");
        assert!(json.contains("\"type\":\"can_analyze\""));
        assert!(json.contains("test.py"));

        let deserialized: PluginMessage =
            serde_json::from_str(&json).expect("Failed to deserialize PluginMessage::CanAnalyze");

        match deserialized {
            PluginMessage::CanAnalyze {
                file_path,
                content_preview,
            } => {
                assert_eq!(file_path, PathBuf::from("test.py"));
                assert_eq!(content_preview, "print('hello')");
            }
            _ => panic!("Expected CanAnalyze message"),
        }
    }

    #[test]
    fn test_plugin_message_get_info_serialization() {
        let message = PluginMessage::GetInfo;

        let json =
            serde_json::to_string(&message).expect("Failed to serialize PluginMessage::GetInfo");
        assert!(json.contains("\"type\":\"get_info\""));

        let deserialized: PluginMessage =
            serde_json::from_str(&json).expect("Failed to deserialize PluginMessage::GetInfo");

        match deserialized {
            PluginMessage::GetInfo => {
                // Success - this is the expected variant
            }
            _ => panic!("Expected GetInfo message"),
        }
    }

    #[test]
    fn test_plugin_response_success_serialization() {
        let response = PluginResponse::Success {
            cache_file: "analysis_12345.json".to_string(),
            processing_time_ms: 250,
        };

        let json =
            serde_json::to_string(&response).expect("Failed to serialize PluginResponse::Success");
        assert!(json.contains("\"status\":\"success\""));
        assert!(json.contains("analysis_12345.json"));

        let deserialized: PluginResponse =
            serde_json::from_str(&json).expect("Failed to deserialize PluginResponse::Success");

        match deserialized {
            PluginResponse::Success {
                cache_file,
                processing_time_ms,
            } => {
                assert_eq!(cache_file, "analysis_12345.json");
                assert_eq!(processing_time_ms, 250);
            }
            _ => panic!("Expected Success response"),
        }
    }

    #[test]
    fn test_plugin_response_can_analyze_serialization() {
        let response = PluginResponse::CanAnalyze {
            can_analyze: true,
            confidence: 0.95,
        };

        let json = serde_json::to_string(&response)
            .expect("Failed to serialize PluginResponse::CanAnalyze");
        assert!(json.contains("\"status\":\"can_analyze\""));
        assert!(json.contains("true"));

        let deserialized: PluginResponse =
            serde_json::from_str(&json).expect("Failed to deserialize PluginResponse::CanAnalyze");

        match deserialized {
            PluginResponse::CanAnalyze {
                can_analyze,
                confidence,
            } => {
                assert!(can_analyze);
                assert_eq!(confidence, 0.95);
            }
            _ => panic!("Expected CanAnalyze response"),
        }
    }

    #[test]
    fn test_plugin_response_info_serialization() {
        let response = PluginResponse::Info {
            name: "python_analyzer".to_string(),
            version: "1.2.0".to_string(),
            supported_extensions: vec![".py".to_string(), ".pyx".to_string()],
            supported_filenames: vec!["requirements.txt".to_string()],
        };

        let json =
            serde_json::to_string(&response).expect("Failed to serialize PluginResponse::Info");
        assert!(json.contains("\"status\":\"info\""));
        assert!(json.contains("python_analyzer"));

        let deserialized: PluginResponse =
            serde_json::from_str(&json).expect("Failed to deserialize PluginResponse::Info");

        match deserialized {
            PluginResponse::Info {
                name,
                version,
                supported_extensions,
                supported_filenames,
            } => {
                assert_eq!(name, "python_analyzer");
                assert_eq!(version, "1.2.0");
                assert!(supported_extensions.contains(&".py".to_string()));
                assert!(supported_filenames.contains(&"requirements.txt".to_string()));
            }
            _ => panic!("Expected Info response"),
        }
    }

    #[test]
    fn test_plugin_response_error_serialization() {
        let response = PluginResponse::Error {
            message: "File not found".to_string(),
            details: Some("Could not read /path/to/file.py".to_string()),
        };

        let json =
            serde_json::to_string(&response).expect("Failed to serialize PluginResponse::Error");
        assert!(json.contains("\"status\":\"error\""));
        assert!(json.contains("File not found"));

        let deserialized: PluginResponse =
            serde_json::from_str(&json).expect("Failed to deserialize PluginResponse::Error");

        match deserialized {
            PluginResponse::Error { message, details } => {
                assert_eq!(message, "File not found");
                assert_eq!(details, Some("Could not read /path/to/file.py".to_string()));
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[test]
    fn test_plugin_info_creation() {
        let info = PluginInfo {
            name: "rust_analyzer".to_string(),
            version: "2.0.0".to_string(),
            supported_extensions: vec![".rs".to_string()],
            supported_filenames: vec!["Cargo.toml".to_string(), "Cargo.lock".to_string()],
        };

        assert_eq!(info.name, "rust_analyzer");
        assert_eq!(info.version, "2.0.0");
        assert_eq!(info.supported_extensions.len(), 1);
        assert_eq!(info.supported_filenames.len(), 2);
    }

    #[test]
    fn test_code_element_with_empty_collections() {
        let element = CodeElement {
            element_type: "variable".to_string(),
            name: "my_var".to_string(),
            signature: None,
            line_start: 5,
            line_end: 5,
            summary: None,
            complexity_score: None,
            calls: vec![], // Empty calls
            metadata: serde_json::Value::Null,
        };

        // Should serialize/deserialize without issues
        let json = serde_json::to_string(&element).expect("Failed to serialize empty CodeElement");
        let deserialized: CodeElement =
            serde_json::from_str(&json).expect("Failed to deserialize empty CodeElement");

        assert_eq!(deserialized.name, "my_var");
        assert_eq!(deserialized.calls.len(), 0);
        assert!(deserialized.summary.is_none());
        assert!(deserialized.complexity_score.is_none());
    }

    #[test]
    fn test_import_with_no_items() {
        let import = Import {
            module: "std::io".to_string(),
            items: vec![], // No specific items imported
            alias: None,
            line_number: 1,
            import_type: "standard".to_string(),
        };

        let json =
            serde_json::to_string(&import).expect("Failed to serialize Import with no items");
        let deserialized: Import =
            serde_json::from_str(&json).expect("Failed to deserialize Import with no items");

        assert_eq!(deserialized.module, "std::io");
        assert_eq!(deserialized.items.len(), 0);
        assert!(deserialized.alias.is_none());
    }

    #[test]
    fn test_relationship_with_no_line_number() {
        let relationship = Relationship {
            from_file: "module_a.rs".to_string(),
            to_file: "module_b.rs".to_string(),
            relationship_type: "call".to_string(),
            details: "function call".to_string(),
            line_number: None, // No specific line number
            strength: 1.0,
        };

        let json = serde_json::to_string(&relationship).expect("Failed to serialize Relationship");
        let deserialized: Relationship =
            serde_json::from_str(&json).expect("Failed to deserialize Relationship");

        assert_eq!(deserialized.relationship_type, "call");
        assert!(deserialized.line_number.is_none());
        assert_eq!(deserialized.strength, 1.0);
    }

    #[test]
    fn test_plugin_input_with_no_config() {
        let input = PluginInput {
            file_path: PathBuf::from("simple.rs"),
            relative_path: PathBuf::from("simple.rs"),
            content: "fn main() {}".to_string(),
            project_root: PathBuf::from("."),
            cache_dir: ".cache".to_string(),
            plugin_config: None, // No plugin configuration
        };

        let json = serde_json::to_string(&input).expect("Failed to serialize PluginInput");
        let deserialized: PluginInput =
            serde_json::from_str(&json).expect("Failed to deserialize PluginInput");

        assert_eq!(deserialized.file_path, PathBuf::from("simple.rs"));
        assert!(deserialized.plugin_config.is_none());
    }
}
