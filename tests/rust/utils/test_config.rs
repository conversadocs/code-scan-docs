use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

// Import the modules we're testing
use csd::utils::config::{
    Config, FilePatterns, InputPluginConfig, LlmConfig, OutputPluginConfig, PluginSource,
    ScanConfig,
};

// Helper function to create a test config with custom plugins
fn create_test_config_with_plugins() -> Config {
    let mut config = Config::default();

    // Add a custom JavaScript input plugin for testing
    config.input_plugins.insert(
        "javascript".to_string(),
        InputPluginConfig {
            source: PluginSource::Builtin {
                name: "javascript".to_string(),
                plugin_type: "code".to_string(),
            },
            file_patterns: FilePatterns {
                extensions: vec![".js".to_string(), ".jsx".to_string()],
                filenames: vec!["package.json".to_string()],
                glob_patterns: Some(vec!["**/*.js".to_string()]),
            },
            enabled: true,
            config: None,
        },
    );

    // Add a custom HTML documentation output plugin for testing
    config.output_plugins.insert(
        "html_docs".to_string(),
        OutputPluginConfig {
            source: PluginSource::Builtin {
                name: "html_docs".to_string(),
                plugin_type: "code".to_string(),
            },
            output_types: vec!["documentation".to_string()],
            formats: vec!["html".to_string()],
            enabled: true,
            config: None,
        },
    );

    config
}

#[test]
fn test_config_default() {
    let config = Config::default();

    // Test basic properties
    assert_eq!(config.output_dir, "output");
    assert_eq!(config.llm.provider, "ollama");
    assert_eq!(config.llm.model, "deepseek-coder");
    assert_eq!(config.llm.timeout_seconds, 30);
    assert_eq!(config.python_executable, None);

    // Test scanning defaults
    assert!(!config.scanning.include_hidden);
    assert_eq!(config.scanning.max_file_size_mb, 10);
    assert!(config
        .scanning
        .ignore_patterns
        .contains(&"target/".to_string()));
    assert!(config
        .scanning
        .ignore_patterns
        .contains(&".csd_cache/".to_string()));

    // Test default input plugins
    assert!(config.input_plugins.contains_key("python"));
    assert!(config.input_plugins.contains_key("rust"));

    let python_plugin = &config.input_plugins["python"];
    assert!(python_plugin.enabled);
    assert!(python_plugin
        .file_patterns
        .extensions
        .contains(&".py".to_string()));
    assert!(python_plugin
        .file_patterns
        .filenames
        .contains(&"requirements.txt".to_string()));

    let rust_plugin = &config.input_plugins["rust"];
    assert!(rust_plugin.enabled);
    assert!(rust_plugin
        .file_patterns
        .extensions
        .contains(&".rs".to_string()));
    assert!(rust_plugin
        .file_patterns
        .filenames
        .contains(&"Cargo.toml".to_string()));

    // Test default output plugins
    assert!(config.output_plugins.contains_key("markdown_docs"));

    let markdown_plugin = &config.output_plugins["markdown_docs"];
    assert!(markdown_plugin.enabled);
    assert!(markdown_plugin
        .output_types
        .contains(&"documentation".to_string()));
    assert!(markdown_plugin.formats.contains(&"markdown".to_string()));

    // Test that legacy plugins field is None by default
    assert!(config.plugins.is_none());
}

#[tokio::test]
async fn test_config_save_and_load() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("test_config.yaml");

    // Create and save a config
    let original_config = create_test_config_with_plugins();
    original_config
        .save(&config_path)
        .await
        .expect("Failed to save config");

    // Verify file was created
    assert!(config_path.exists());

    // Load the config back
    let loaded_config = Config::load(&config_path)
        .await
        .expect("Failed to load config");

    // Verify the loaded config matches the original
    assert_eq!(loaded_config.output_dir, original_config.output_dir);
    assert_eq!(loaded_config.llm.provider, original_config.llm.provider);
    assert_eq!(loaded_config.llm.model, original_config.llm.model);
    assert_eq!(
        loaded_config.python_executable,
        original_config.python_executable
    );

    // Verify input plugins were preserved
    assert_eq!(
        loaded_config.input_plugins.len(),
        original_config.input_plugins.len()
    );
    assert!(loaded_config.input_plugins.contains_key("javascript"));

    let js_plugin = &loaded_config.input_plugins["javascript"];
    assert!(js_plugin.enabled);
    assert!(js_plugin
        .file_patterns
        .extensions
        .contains(&".js".to_string()));

    // Verify output plugins were preserved
    assert_eq!(
        loaded_config.output_plugins.len(),
        original_config.output_plugins.len()
    );
    assert!(loaded_config.output_plugins.contains_key("html_docs"));

    let html_plugin = &loaded_config.output_plugins["html_docs"];
    assert!(html_plugin.enabled);
    assert!(html_plugin
        .output_types
        .contains(&"documentation".to_string()));
    assert!(html_plugin.formats.contains(&"html".to_string()));
}

#[tokio::test]
async fn test_config_load_invalid_yaml() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("invalid_config.yaml");

    // Write invalid YAML
    let invalid_yaml = r#"
output_dir: "test"
invalid_yaml: [unclosed bracket
input_plugins:
- this is not: valid yaml structure
"#;
    fs::write(&config_path, invalid_yaml)
        .await
        .expect("Failed to write invalid config");

    let result = Config::load(&config_path).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_config_save_to_readonly_location() {
    // Try to save to a location that should fail (root directory)
    let readonly_path = PathBuf::from("/root/readonly_config.yaml");
    let config = Config::default();

    let result = config.save(&readonly_path).await;
    assert!(result.is_err());
}

#[test]
fn test_find_input_plugin_for_file_by_extension() {
    let config = create_test_config_with_plugins();

    // Test Python files
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("src/main.py")),
        Some("python".to_string())
    );
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("script.py")),
        Some("python".to_string())
    );

    // Test Rust files
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("src/main.rs")),
        Some("rust".to_string())
    );

    // Test JavaScript files (from our custom plugin)
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("app.js")),
        Some("javascript".to_string())
    );
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("component.jsx")),
        Some("javascript".to_string())
    );

    // Test unknown extension
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("README.md")),
        None
    );
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("image.png")),
        None
    );
}

#[test]
fn test_find_input_plugin_for_file_by_filename() {
    let config = create_test_config_with_plugins();

    // Test Python ecosystem files
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("requirements.txt")),
        Some("python".to_string())
    );
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("setup.py")),
        Some("python".to_string())
    );
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("pyproject.toml")),
        Some("python".to_string())
    );

    // Test Rust ecosystem files
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("Cargo.toml")),
        Some("rust".to_string())
    );
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("Cargo.lock")),
        Some("rust".to_string())
    );

    // Test JavaScript ecosystem files (from our custom plugin)
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("package.json")),
        Some("javascript".to_string())
    );
}

#[test]
fn test_find_output_plugins_for_type() {
    let config = create_test_config_with_plugins();

    // Test finding documentation plugins
    let doc_plugins_md = config.find_output_plugins_for_type("documentation", "markdown");
    assert!(!doc_plugins_md.is_empty());
    assert!(doc_plugins_md.contains(&"markdown_docs".to_string()));

    let doc_plugins_html = config.find_output_plugins_for_type("documentation", "html");
    assert!(!doc_plugins_html.is_empty());
    assert!(doc_plugins_html.contains(&"html_docs".to_string()));

    // Test unknown output type
    let unknown_plugins = config.find_output_plugins_for_type("unknown_type", "unknown_format");
    assert!(unknown_plugins.is_empty());

    // Test wrong format for existing type
    let wrong_format = config.find_output_plugins_for_type("documentation", "pdf");
    // Should be empty since no plugins support PDF in our test config
    assert!(wrong_format.is_empty());
}

#[test]
fn test_find_plugin_for_file_case_sensitivity() {
    let config = create_test_config_with_plugins();

    // Test case variations (legacy method for compatibility)
    assert_eq!(
        config.find_plugin_for_file(&PathBuf::from("REQUIREMENTS.TXT")),
        Some("python".to_string())
    );
    assert_eq!(
        config.find_plugin_for_file(&PathBuf::from("Requirements.txt")),
        Some("python".to_string())
    );
    assert_eq!(
        config.find_plugin_for_file(&PathBuf::from("cargo.toml")),
        Some("rust".to_string())
    );
    assert_eq!(
        config.find_plugin_for_file(&PathBuf::from("CARGO.TOML")),
        Some("rust".to_string())
    );
}

#[test]
fn test_find_plugin_for_file_with_path() {
    let config = create_test_config_with_plugins();

    // Test files in subdirectories
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("src/utils/helper.py")),
        Some("python".to_string())
    );
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("backend/core/main.rs")),
        Some("rust".to_string())
    );
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("frontend/src/app.js")),
        Some("javascript".to_string())
    );

    // Test absolute paths
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("/home/user/project/script.py")),
        Some("python".to_string())
    );
}

#[test]
fn test_find_plugin_for_file_disabled_plugin() {
    let mut config = create_test_config_with_plugins();

    // Disable the JavaScript plugin
    config.input_plugins.get_mut("javascript").unwrap().enabled = false;

    // Should not match disabled plugin
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("app.js")),
        None
    );
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("package.json")),
        None
    );

    // But enabled plugins should still work
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("main.py")),
        Some("python".to_string())
    );
}

#[test]
fn test_find_plugin_for_file_no_extension() {
    let config = create_test_config_with_plugins();

    // Files without extensions should not match extension patterns
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("README")),
        None
    );
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("LICENSE")),
        None
    );
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("Makefile")),
        None
    );

    // But should match filename patterns
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("requirements.txt")),
        Some("python".to_string())
    );
}

#[test]
fn test_plugin_source_types() {
    let config = Config::default();

    // Test that default input plugins use builtin source
    let python_plugin = &config.input_plugins["python"];
    match &python_plugin.source {
        PluginSource::Builtin { name, plugin_type } => {
            assert_eq!(name, "python_analyzer");
            assert_eq!(plugin_type, "code");
        }
        _ => panic!("Expected builtin source for python plugin"),
    }

    let rust_plugin = &config.input_plugins["rust"];
    match &rust_plugin.source {
        PluginSource::Builtin { name, plugin_type } => {
            assert_eq!(name, "rust_analyzer");
            assert_eq!(plugin_type, "code");
        }
        _ => panic!("Expected builtin source for rust plugin"),
    }

    // Test that default output plugins use builtin source
    let markdown_plugin = &config.output_plugins["markdown_docs"];
    match &markdown_plugin.source {
        PluginSource::Builtin { name, plugin_type } => {
            assert_eq!(name, "markdown_docs");
            assert_eq!(plugin_type, "docs");
        }
        _ => panic!("Expected builtin source for markdown_docs plugin"),
    }
}

#[test]
fn test_file_patterns_structure() {
    let config = Config::default();
    let python_plugin = &config.input_plugins["python"];

    // Test that file patterns are properly structured
    assert!(!python_plugin.file_patterns.extensions.is_empty());
    assert!(!python_plugin.file_patterns.filenames.is_empty());
    assert!(python_plugin.file_patterns.glob_patterns.is_some());

    // Test specific patterns
    let patterns = &python_plugin.file_patterns;
    assert!(patterns.extensions.contains(&".py".to_string()));
    assert!(patterns.filenames.contains(&"requirements.txt".to_string()));

    if let Some(ref globs) = patterns.glob_patterns {
        assert!(globs.iter().any(|p| p.contains("requirements")));
    }
}

#[test]
fn test_plugin_management_methods() {
    let mut config = Config::default();

    // Test get methods
    assert!(config.get_input_plugin("python").is_some());
    assert!(config.get_output_plugin("markdown_docs").is_some());
    assert!(config.get_input_plugin("nonexistent").is_none());

    // Test enabled plugins
    let enabled_input = config.get_enabled_input_plugins();
    let enabled_output = config.get_enabled_output_plugins();

    assert!(!enabled_input.is_empty());
    assert!(!enabled_output.is_empty());

    // Test adding new plugins
    let new_input_plugin = InputPluginConfig {
        source: PluginSource::Local {
            path: "/path/to/plugin".to_string(),
        },
        file_patterns: FilePatterns {
            extensions: vec![".test".to_string()],
            filenames: vec![],
            glob_patterns: None,
        },
        enabled: true,
        config: None,
    };

    config.add_input_plugin("test_plugin".to_string(), new_input_plugin);
    assert!(config.get_input_plugin("test_plugin").is_some());

    // Test removing plugins
    let removed = config.remove_input_plugin("test_plugin");
    assert!(removed.is_some());
    assert!(config.get_input_plugin("test_plugin").is_none());
}

#[test]
fn test_plugin_summary() {
    let config = create_test_config_with_plugins();
    let summary = config.get_plugin_summary();

    // Should have at least the default plugins plus our custom ones
    assert!(summary.total_input_plugins >= 3); // python, rust, javascript
    assert!(summary.total_output_plugins >= 2); // markdown_docs, html_docs

    // All should be enabled in our test config
    assert_eq!(summary.enabled_input_plugins, summary.total_input_plugins);
    assert_eq!(summary.enabled_output_plugins, summary.total_output_plugins);

    // Check plugin names
    assert!(summary.input_plugin_names.contains(&"python".to_string()));
    assert!(summary.input_plugin_names.contains(&"rust".to_string()));
    assert!(summary
        .input_plugin_names
        .contains(&"javascript".to_string()));
    assert!(summary
        .output_plugin_names
        .contains(&"markdown_docs".to_string()));
    assert!(summary
        .output_plugin_names
        .contains(&"html_docs".to_string()));
}

#[tokio::test]
async fn test_config_roundtrip_preserves_data() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("roundtrip_config.yaml");

    // Create a config with specific values
    let original_config = Config {
        output_dir: "custom_output".to_string(),
        python_executable: Some("/usr/bin/python3.11".to_string()),
        llm: LlmConfig {
            timeout_seconds: 120,
            ..Config::default().llm
        },
        scanning: ScanConfig {
            max_file_size_mb: 25,
            ..Config::default().scanning
        },
        ..Config::default()
    };

    // Save and reload
    original_config
        .save(&config_path)
        .await
        .expect("Failed to save");
    let loaded_config = Config::load(&config_path).await.expect("Failed to load");

    // Verify all custom values are preserved
    assert_eq!(loaded_config.output_dir, "custom_output");
    assert_eq!(
        loaded_config.python_executable,
        Some("/usr/bin/python3.11".to_string())
    );
    assert_eq!(loaded_config.llm.timeout_seconds, 120);
    assert_eq!(loaded_config.scanning.max_file_size_mb, 25);

    // Verify input plugin data is preserved
    assert_eq!(
        loaded_config.input_plugins.len(),
        original_config.input_plugins.len()
    );
    for (key, original_plugin) in &original_config.input_plugins {
        let loaded_plugin = loaded_config
            .input_plugins
            .get(key)
            .expect("Input plugin missing");
        assert_eq!(loaded_plugin.enabled, original_plugin.enabled);
        assert_eq!(
            loaded_plugin.file_patterns.extensions.len(),
            original_plugin.file_patterns.extensions.len()
        );
    }

    // Verify output plugin data is preserved
    assert_eq!(
        loaded_config.output_plugins.len(),
        original_config.output_plugins.len()
    );
    for (key, original_plugin) in &original_config.output_plugins {
        let loaded_plugin = loaded_config
            .output_plugins
            .get(key)
            .expect("Output plugin missing");
        assert_eq!(loaded_plugin.enabled, original_plugin.enabled);
        assert_eq!(
            loaded_plugin.output_types.len(),
            original_plugin.output_types.len()
        );
    }
}
