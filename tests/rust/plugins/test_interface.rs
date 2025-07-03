use serde_json;
use std::path::PathBuf;

// Import the modules we're testing
use csd::plugins::interface::{
    CodeElement, ExternalDependency, GeneratedOutput, Import, OutputPluginInput,
    OutputPluginResult, PluginInfo, PluginInput, PluginMessage, PluginOutput, PluginResponse,
    PluginType, Relationship,
};

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

// Helper function to create a test OutputPluginInput
fn create_test_output_plugin_input() -> OutputPluginInput {
    OutputPluginInput {
        matrix_path: PathBuf::from("/project/.csd_cache/matrix.json"),
        project_root: PathBuf::from("/project"),
        output_dir: PathBuf::from("/project/docs"),
        cache_dir: "/project/.csd_cache".to_string(),
        plugin_config: Some(serde_json::json!({
            "include_toc": true,
            "theme": "modern"
        })),
        format_options: serde_json::json!({
            "format": "markdown",
            "output_type": "documentation"
        }),
    }
}

// Helper function to create a test GeneratedOutput
fn create_test_generated_output() -> GeneratedOutput {
    GeneratedOutput {
        output_path: PathBuf::from("/project/docs/README.md"),
        content_type: "markdown".to_string(),
        size_bytes: 2048,
        checksum: "sha256:def789abc123".to_string(),
        metadata: serde_json::json!({
            "sections": ["overview", "api", "examples"],
            "word_count": 350
        }),
    }
}

// Helper function to create a test OutputPluginResult
fn create_test_output_plugin_result() -> OutputPluginResult {
    OutputPluginResult {
        plugin_name: "markdown_docs".to_string(),
        plugin_version: "1.0.0".to_string(),
        output_type: "documentation".to_string(),
        outputs: vec![create_test_generated_output()],
        processing_time_ms: 500,
        metadata: serde_json::json!({
            "total_files": 1,
            "total_size_mb": 0.002
        }),
    }
}

#[test]
fn test_plugin_type_serialization() {
    let input_type = PluginType::Input;
    let output_type = PluginType::Output;

    let input_json = serde_json::to_string(&input_type).expect("Failed to serialize Input type");
    let output_json = serde_json::to_string(&output_type).expect("Failed to serialize Output type");

    assert_eq!(input_json, "\"input\"");
    assert_eq!(output_json, "\"output\"");

    let deserialized_input: PluginType =
        serde_json::from_str(&input_json).expect("Failed to deserialize Input type");
    let deserialized_output: PluginType =
        serde_json::from_str(&output_json).expect("Failed to deserialize Output type");

    assert_eq!(deserialized_input, PluginType::Input);
    assert_eq!(deserialized_output, PluginType::Output);
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
    let deserialized: Import = serde_json::from_str(&json).expect("Failed to deserialize Import");

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

    let json = serde_json::to_string(&dependency).expect("Failed to serialize ExternalDependency");
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
fn test_output_plugin_input_creation() {
    let input = create_test_output_plugin_input();

    assert_eq!(
        input.matrix_path,
        PathBuf::from("/project/.csd_cache/matrix.json")
    );
    assert_eq!(input.project_root, PathBuf::from("/project"));
    assert_eq!(input.output_dir, PathBuf::from("/project/docs"));
    assert_eq!(input.cache_dir, "/project/.csd_cache");
    assert!(input.plugin_config.is_some());
    assert!(input.format_options.is_object());
}

#[test]
fn test_output_plugin_input_serialization() {
    let input = create_test_output_plugin_input();

    let json = serde_json::to_string(&input).expect("Failed to serialize OutputPluginInput");
    let deserialized: OutputPluginInput =
        serde_json::from_str(&json).expect("Failed to deserialize OutputPluginInput");

    assert_eq!(deserialized.matrix_path, input.matrix_path);
    assert_eq!(deserialized.output_dir, input.output_dir);
    assert_eq!(deserialized.cache_dir, input.cache_dir);
}

#[test]
fn test_generated_output_creation() {
    let output = create_test_generated_output();

    assert_eq!(output.output_path, PathBuf::from("/project/docs/README.md"));
    assert_eq!(output.content_type, "markdown");
    assert_eq!(output.size_bytes, 2048);
    assert_eq!(output.checksum, "sha256:def789abc123");
    assert!(output.metadata.is_object());
}

#[test]
fn test_generated_output_serialization() {
    let output = create_test_generated_output();

    let json = serde_json::to_string(&output).expect("Failed to serialize GeneratedOutput");
    let deserialized: GeneratedOutput =
        serde_json::from_str(&json).expect("Failed to deserialize GeneratedOutput");

    assert_eq!(deserialized.output_path, output.output_path);
    assert_eq!(deserialized.content_type, output.content_type);
    assert_eq!(deserialized.size_bytes, output.size_bytes);
    assert_eq!(deserialized.checksum, output.checksum);
}

#[test]
fn test_output_plugin_result_creation() {
    let result = create_test_output_plugin_result();

    assert_eq!(result.plugin_name, "markdown_docs");
    assert_eq!(result.plugin_version, "1.0.0");
    assert_eq!(result.output_type, "documentation");
    assert_eq!(result.outputs.len(), 1);
    assert_eq!(result.processing_time_ms, 500);
    assert!(result.metadata.is_object());
}

#[test]
fn test_output_plugin_result_serialization() {
    let result = create_test_output_plugin_result();

    let json = serde_json::to_string(&result).expect("Failed to serialize OutputPluginResult");
    let deserialized: OutputPluginResult =
        serde_json::from_str(&json).expect("Failed to deserialize OutputPluginResult");

    assert_eq!(deserialized.plugin_name, result.plugin_name);
    assert_eq!(deserialized.output_type, result.output_type);
    assert_eq!(deserialized.outputs.len(), result.outputs.len());
    assert_eq!(deserialized.processing_time_ms, result.processing_time_ms);
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

    let json = serde_json::to_string(&message).expect("Failed to serialize PluginMessage::Analyze");
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
fn test_plugin_message_generate_serialization() {
    let input = create_test_output_plugin_input();
    let message = PluginMessage::Generate { input };

    let json =
        serde_json::to_string(&message).expect("Failed to serialize PluginMessage::Generate");
    assert!(json.contains("\"type\":\"generate\""));
    assert!(json.contains("matrix.json"));

    let deserialized: PluginMessage =
        serde_json::from_str(&json).expect("Failed to deserialize PluginMessage::Generate");

    match deserialized {
        PluginMessage::Generate {
            input: deserialized_input,
        } => {
            assert_eq!(
                deserialized_input.matrix_path,
                PathBuf::from("/project/.csd_cache/matrix.json")
            );
            assert_eq!(deserialized_input.cache_dir, "/project/.csd_cache");
        }
        _ => panic!("Expected Generate message"),
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
fn test_plugin_message_can_generate_serialization() {
    let message = PluginMessage::CanGenerate {
        output_type: "documentation".to_string(),
        format: "markdown".to_string(),
    };

    let json =
        serde_json::to_string(&message).expect("Failed to serialize PluginMessage::CanGenerate");
    assert!(json.contains("\"type\":\"can_generate\""));
    assert!(json.contains("documentation"));
    assert!(json.contains("markdown"));

    let deserialized: PluginMessage =
        serde_json::from_str(&json).expect("Failed to deserialize PluginMessage::CanGenerate");

    match deserialized {
        PluginMessage::CanGenerate {
            output_type,
            format,
        } => {
            assert_eq!(output_type, "documentation");
            assert_eq!(format, "markdown");
        }
        _ => panic!("Expected CanGenerate message"),
    }
}

#[test]
fn test_plugin_message_get_info_serialization() {
    let message = PluginMessage::GetInfo;

    let json = serde_json::to_string(&message).expect("Failed to serialize PluginMessage::GetInfo");
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
fn test_plugin_response_output_success_serialization() {
    let result = create_test_output_plugin_result();
    let response = PluginResponse::OutputSuccess { result };

    let json = serde_json::to_string(&response)
        .expect("Failed to serialize PluginResponse::OutputSuccess");
    assert!(json.contains("\"status\":\"output_success\""));
    assert!(json.contains("markdown_docs"));

    let deserialized: PluginResponse =
        serde_json::from_str(&json).expect("Failed to deserialize PluginResponse::OutputSuccess");

    match deserialized {
        PluginResponse::OutputSuccess { result } => {
            assert_eq!(result.plugin_name, "markdown_docs");
            assert_eq!(result.output_type, "documentation");
            assert_eq!(result.outputs.len(), 1);
        }
        _ => panic!("Expected OutputSuccess response"),
    }
}

#[test]
fn test_plugin_response_can_analyze_serialization() {
    let response = PluginResponse::CanAnalyze {
        can_analyze: true,
        confidence: 0.95,
    };

    let json =
        serde_json::to_string(&response).expect("Failed to serialize PluginResponse::CanAnalyze");
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
fn test_plugin_response_can_generate_serialization() {
    let response = PluginResponse::CanGenerate {
        can_generate: true,
        confidence: 0.9,
    };

    let json =
        serde_json::to_string(&response).expect("Failed to serialize PluginResponse::CanGenerate");
    assert!(json.contains("\"status\":\"can_generate\""));
    assert!(json.contains("true"));

    let deserialized: PluginResponse =
        serde_json::from_str(&json).expect("Failed to deserialize PluginResponse::CanGenerate");

    match deserialized {
        PluginResponse::CanGenerate {
            can_generate,
            confidence,
        } => {
            assert!(can_generate);
            assert_eq!(confidence, 0.9);
        }
        _ => panic!("Expected CanGenerate response"),
    }
}

#[test]
fn test_plugin_response_info_serialization() {
    let response = PluginResponse::Info {
        name: "python_analyzer".to_string(),
        version: "1.2.0".to_string(),
        plugin_type: PluginType::Input,
        supported_extensions: vec![".py".to_string(), ".pyx".to_string()],
        supported_filenames: vec!["requirements.txt".to_string()],
        supported_output_types: None,
        supported_formats: None,
    };

    let json = serde_json::to_string(&response).expect("Failed to serialize PluginResponse::Info");
    assert!(json.contains("\"status\":\"info\""));
    assert!(json.contains("python_analyzer"));
    assert!(json.contains("\"input\""));

    let deserialized: PluginResponse =
        serde_json::from_str(&json).expect("Failed to deserialize PluginResponse::Info");

    match deserialized {
        PluginResponse::Info {
            name,
            version,
            plugin_type,
            supported_extensions,
            supported_filenames,
            supported_output_types,
            supported_formats,
        } => {
            assert_eq!(name, "python_analyzer");
            assert_eq!(version, "1.2.0");
            assert_eq!(plugin_type, PluginType::Input);
            assert!(supported_extensions.contains(&".py".to_string()));
            assert!(supported_filenames.contains(&"requirements.txt".to_string()));
            assert!(supported_output_types.is_none());
            assert!(supported_formats.is_none());
        }
        _ => panic!("Expected Info response"),
    }
}

#[test]
fn test_plugin_response_info_output_plugin_serialization() {
    let response = PluginResponse::Info {
        name: "markdown_generator".to_string(),
        version: "2.0.0".to_string(),
        plugin_type: PluginType::Output,
        supported_extensions: vec![],
        supported_filenames: vec![],
        supported_output_types: Some(vec!["documentation".to_string(), "reports".to_string()]),
        supported_formats: Some(vec!["markdown".to_string(), "html".to_string()]),
    };

    let json = serde_json::to_string(&response).expect("Failed to serialize output plugin Info");
    assert!(json.contains("\"status\":\"info\""));
    assert!(json.contains("markdown_generator"));
    assert!(json.contains("\"output\""));
    assert!(json.contains("documentation"));

    let deserialized: PluginResponse =
        serde_json::from_str(&json).expect("Failed to deserialize output plugin Info");

    match deserialized {
        PluginResponse::Info {
            name,
            plugin_type,
            supported_output_types,
            supported_formats,
            ..
        } => {
            assert_eq!(name, "markdown_generator");
            assert_eq!(plugin_type, PluginType::Output);
            assert!(supported_output_types.is_some());
            assert!(supported_formats.is_some());
            let output_types = supported_output_types.unwrap();
            let formats = supported_formats.unwrap();
            assert!(output_types.contains(&"documentation".to_string()));
            assert!(formats.contains(&"markdown".to_string()));
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

    let json = serde_json::to_string(&response).expect("Failed to serialize PluginResponse::Error");
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
        plugin_type: PluginType::Input,
        supported_extensions: vec![".rs".to_string()],
        supported_filenames: vec!["Cargo.toml".to_string(), "Cargo.lock".to_string()],
        supported_output_types: None,
        supported_formats: None,
    };

    assert_eq!(info.name, "rust_analyzer");
    assert_eq!(info.version, "2.0.0");
    assert_eq!(info.plugin_type, PluginType::Input);
    assert_eq!(info.supported_extensions.len(), 1);
    assert_eq!(info.supported_filenames.len(), 2);
    assert!(info.is_input_plugin());
    assert!(!info.is_output_plugin());
}

#[test]
fn test_plugin_info_output_plugin() {
    let info = PluginInfo {
        name: "doc_generator".to_string(),
        version: "1.5.0".to_string(),
        plugin_type: PluginType::Output,
        supported_extensions: vec![],
        supported_filenames: vec![],
        supported_output_types: Some(vec!["documentation".to_string()]),
        supported_formats: Some(vec!["markdown".to_string(), "html".to_string()]),
    };

    assert_eq!(info.name, "doc_generator");
    assert_eq!(info.plugin_type, PluginType::Output);
    assert!(!info.is_input_plugin());
    assert!(info.is_output_plugin());

    let capabilities = info.get_capabilities_description();
    assert!(capabilities.contains("Types: documentation"));
    assert!(capabilities.contains("Formats: markdown, html"));
}

#[test]
fn test_plugin_info_capabilities_description() {
    // Test input plugin capabilities
    let input_info = PluginInfo {
        name: "python".to_string(),
        version: "1.0.0".to_string(),
        plugin_type: PluginType::Input,
        supported_extensions: vec![".py".to_string(), ".pyx".to_string()],
        supported_filenames: vec!["requirements.txt".to_string()],
        supported_output_types: None,
        supported_formats: None,
    };

    let input_caps = input_info.get_capabilities_description();
    assert!(input_caps.contains("Extensions: .py, .pyx"));
    assert!(input_caps.contains("Files: requirements.txt"));

    // Test output plugin capabilities
    let output_info = PluginInfo {
        name: "docs".to_string(),
        version: "1.0.0".to_string(),
        plugin_type: PluginType::Output,
        supported_extensions: vec![],
        supported_filenames: vec![],
        supported_output_types: Some(vec!["documentation".to_string(), "reports".to_string()]),
        supported_formats: Some(vec!["markdown".to_string(), "pdf".to_string()]),
    };

    let output_caps = output_info.get_capabilities_description();
    assert!(output_caps.contains("Types: documentation, reports"));
    assert!(output_caps.contains("Formats: markdown, pdf"));
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

    let json = serde_json::to_string(&import).expect("Failed to serialize Import with no items");
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

#[test]
fn test_output_plugin_input_with_minimal_config() {
    let input = OutputPluginInput {
        matrix_path: PathBuf::from("matrix.json"),
        project_root: PathBuf::from("."),
        output_dir: PathBuf::from("./output"),
        cache_dir: ".cache".to_string(),
        plugin_config: None,
        format_options: serde_json::Value::Null,
    };

    let json =
        serde_json::to_string(&input).expect("Failed to serialize minimal OutputPluginInput");
    let deserialized: OutputPluginInput =
        serde_json::from_str(&json).expect("Failed to deserialize minimal OutputPluginInput");

    assert_eq!(deserialized.matrix_path, PathBuf::from("matrix.json"));
    assert!(deserialized.plugin_config.is_none());
    assert!(deserialized.format_options.is_null());
}

#[test]
fn test_generated_output_with_empty_metadata() {
    let output = GeneratedOutput {
        output_path: PathBuf::from("simple.md"),
        content_type: "markdown".to_string(),
        size_bytes: 100,
        checksum: "abc123".to_string(),
        metadata: serde_json::Value::Null,
    };

    let json = serde_json::to_string(&output).expect("Failed to serialize GeneratedOutput");
    let deserialized: GeneratedOutput =
        serde_json::from_str(&json).expect("Failed to deserialize GeneratedOutput");

    assert_eq!(deserialized.output_path, PathBuf::from("simple.md"));
    assert!(deserialized.metadata.is_null());
}

#[test]
fn test_output_plugin_result_with_multiple_outputs() {
    let outputs = vec![
        GeneratedOutput {
            output_path: PathBuf::from("README.md"),
            content_type: "markdown".to_string(),
            size_bytes: 1024,
            checksum: "hash1".to_string(),
            metadata: serde_json::json!({"sections": 3}),
        },
        GeneratedOutput {
            output_path: PathBuf::from("API.md"),
            content_type: "markdown".to_string(),
            size_bytes: 2048,
            checksum: "hash2".to_string(),
            metadata: serde_json::json!({"functions": 15}),
        },
    ];

    let result = OutputPluginResult {
        plugin_name: "multi_doc".to_string(),
        plugin_version: "1.0.0".to_string(),
        output_type: "documentation".to_string(),
        outputs,
        processing_time_ms: 1000,
        metadata: serde_json::json!({"total_files": 2}),
    };

    let json = serde_json::to_string(&result).expect("Failed to serialize multi-output result");
    let deserialized: OutputPluginResult =
        serde_json::from_str(&json).expect("Failed to deserialize multi-output result");

    assert_eq!(deserialized.outputs.len(), 2);
    assert_eq!(
        deserialized.outputs[0].output_path,
        PathBuf::from("README.md")
    );
    assert_eq!(deserialized.outputs[1].output_path, PathBuf::from("API.md"));
}

#[test]
fn test_plugin_message_round_trip() {
    // Test that all message types can be serialized and deserialized
    let messages = vec![
        PluginMessage::Analyze {
            input: create_test_plugin_input(),
        },
        PluginMessage::Generate {
            input: create_test_output_plugin_input(),
        },
        PluginMessage::CanAnalyze {
            file_path: PathBuf::from("test.py"),
            content_preview: "test content".to_string(),
        },
        PluginMessage::CanGenerate {
            output_type: "docs".to_string(),
            format: "html".to_string(),
        },
        PluginMessage::GetInfo,
    ];

    for message in messages {
        let json = serde_json::to_string(&message).expect("Failed to serialize message");
        let deserialized: PluginMessage =
            serde_json::from_str(&json).expect("Failed to deserialize message");

        // Check that the discriminant (type) matches
        match (&message, &deserialized) {
            (PluginMessage::Analyze { .. }, PluginMessage::Analyze { .. }) => {}
            (PluginMessage::Generate { .. }, PluginMessage::Generate { .. }) => {}
            (PluginMessage::CanAnalyze { .. }, PluginMessage::CanAnalyze { .. }) => {}
            (PluginMessage::CanGenerate { .. }, PluginMessage::CanGenerate { .. }) => {}
            (PluginMessage::GetInfo, PluginMessage::GetInfo) => {}
            _ => panic!("Message type mismatch after round trip"),
        }
    }
}

#[test]
fn test_plugin_response_round_trip() {
    // Test that all response types can be serialized and deserialized
    let responses = vec![
        PluginResponse::Success {
            cache_file: "test.json".to_string(),
            processing_time_ms: 100,
        },
        PluginResponse::OutputSuccess {
            result: create_test_output_plugin_result(),
        },
        PluginResponse::CanAnalyze {
            can_analyze: true,
            confidence: 0.8,
        },
        PluginResponse::CanGenerate {
            can_generate: false,
            confidence: 0.1,
        },
        PluginResponse::Info {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            plugin_type: PluginType::Input,
            supported_extensions: vec![".test".to_string()],
            supported_filenames: vec![],
            supported_output_types: None,
            supported_formats: None,
        },
        PluginResponse::Error {
            message: "Test error".to_string(),
            details: None,
        },
    ];

    for response in responses {
        let json = serde_json::to_string(&response).expect("Failed to serialize response");
        let deserialized: PluginResponse =
            serde_json::from_str(&json).expect("Failed to deserialize response");

        // Check that the discriminant (status) matches
        match (&response, &deserialized) {
            (PluginResponse::Success { .. }, PluginResponse::Success { .. }) => {}
            (PluginResponse::OutputSuccess { .. }, PluginResponse::OutputSuccess { .. }) => {}
            (PluginResponse::CanAnalyze { .. }, PluginResponse::CanAnalyze { .. }) => {}
            (PluginResponse::CanGenerate { .. }, PluginResponse::CanGenerate { .. }) => {}
            (PluginResponse::Info { .. }, PluginResponse::Info { .. }) => {}
            (PluginResponse::Error { .. }, PluginResponse::Error { .. }) => {}
            _ => panic!("Response type mismatch after round trip"),
        }
    }
}
