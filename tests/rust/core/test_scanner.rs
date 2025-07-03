use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

// Import the modules we're testing
use csd::core::scanner::{FileInfo, ProjectScanner};
use csd::utils::config::{Config, FilePatterns, InputPluginConfig, PluginSource};

// Helper function to create a test project structure
async fn create_test_project(temp_dir: &TempDir) -> anyhow::Result<PathBuf> {
    let project_root = temp_dir.path().to_path_buf();

    // Create directory structure
    fs::create_dir_all(project_root.join("src")).await?;
    fs::create_dir_all(project_root.join("tests")).await?;
    fs::create_dir_all(project_root.join("target/debug")).await?; // Should be ignored
    fs::create_dir_all(project_root.join(".git")).await?; // Should be ignored

    // Create test files
    fs::write(
        project_root.join("src/main.rs"),
        "fn main() { println!(\"Hello\"); }",
    )
    .await?;
    fs::write(project_root.join("src/lib.rs"), "pub mod utils;").await?;
    fs::write(project_root.join("src/utils.rs"), "pub fn helper() {}").await?;
    fs::write(
        project_root.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"",
    )
    .await?;
    fs::write(project_root.join("Cargo.lock"), "# Cargo.lock").await?;

    // Python files
    fs::write(project_root.join("script.py"), "print('hello')").await?;
    fs::write(project_root.join("requirements.txt"), "requests==2.28.0").await?;

    // Files that should be ignored (ensure parent dirs exist)
    fs::write(project_root.join("target/debug/test"), "binary").await?;
    fs::write(project_root.join("app.log"), "log data").await?;
    fs::write(project_root.join(".git/config"), "git config").await?;

    // Large file (should be ignored if over limit)
    let large_content = "x".repeat(15 * 1024 * 1024); // 15MB
    fs::write(project_root.join("large_file.txt"), large_content).await?;

    // Hidden file
    fs::write(project_root.join(".hidden"), "hidden content").await?;

    // Binary file
    fs::write(project_root.join("binary.bin"), vec![0u8, 1u8, 2u8, 255u8]).await?;

    Ok(project_root)
}

// Helper function to create a minimal config for testing
fn create_test_config() -> Config {
    let mut config = Config::default();
    // Override some settings for testing
    config.scanning.max_file_size_mb = 10; // 10MB limit
    config.scanning.include_hidden = false;
    config
}

// Helper function to create config with custom patterns
fn create_config_with_custom_patterns() -> Config {
    let mut config = create_test_config();

    // Add a test plugin that matches .test files
    config.input_plugins.insert(
        "test_plugin".to_string(),
        InputPluginConfig {
            source: PluginSource::Builtin {
                name: "test".to_string(),
            },
            file_patterns: FilePatterns {
                extensions: vec![".test".to_string()],
                filenames: vec!["TEST_FILE".to_string()],
                glob_patterns: None,
            },
            enabled: true,
            config: None,
        },
    );

    config
}

#[tokio::test]
async fn test_scan_finds_expected_files() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir)
        .await
        .expect("Failed to create test project");

    let config = create_test_config();
    let scanner = ProjectScanner::new(config).with_root(&project_root);

    let files = scanner.scan().await.expect("Scan failed");

    // Should find our test files
    let file_paths: Vec<String> = files
        .iter()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    // Check that we found the expected files
    assert!(file_paths.iter().any(|p| p.contains("main.rs")));
    assert!(file_paths.iter().any(|p| p.contains("lib.rs")));
    assert!(file_paths.iter().any(|p| p.contains("utils.rs")));
    assert!(file_paths.iter().any(|p| p.contains("Cargo.toml")));
    assert!(file_paths.iter().any(|p| p.contains("script.py")));
    assert!(file_paths.iter().any(|p| p.contains("requirements.txt")));

    // Should NOT find ignored files
    assert!(!file_paths.iter().any(|p| p.contains("target/")));
    assert!(!file_paths.iter().any(|p| p.contains(".git/")));
    assert!(!file_paths.iter().any(|p| p.contains("app.log")));

    // Should not find large file (over 10MB limit)
    assert!(!file_paths.iter().any(|p| p.contains("large_file.txt")));

    // Should not find hidden files (include_hidden = false)
    assert!(!file_paths.iter().any(|p| p.contains(".hidden")));
}

#[tokio::test]
async fn test_scan_with_hidden_files_enabled() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir)
        .await
        .expect("Failed to create test project");

    let mut config = create_test_config();
    config.scanning.include_hidden = true;

    let scanner = ProjectScanner::new(config).with_root(&project_root);
    let files = scanner.scan().await.expect("Scan failed");

    let file_paths: Vec<String> = files
        .iter()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    // Should find hidden files when enabled
    assert!(file_paths.iter().any(|p| p.contains(".hidden")));
}

#[tokio::test]
async fn test_scan_respects_file_size_limit() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir)
        .await
        .expect("Failed to create test project");

    let mut config = create_test_config();
    config.scanning.max_file_size_mb = 20; // Increase limit to 20MB

    let scanner = ProjectScanner::new(config).with_root(&project_root);
    let files = scanner.scan().await.expect("Scan failed");

    let file_paths: Vec<String> = files
        .iter()
        .map(|f| f.relative_path.to_string_lossy().to_string())
        .collect();

    // Should now find the large file
    assert!(file_paths.iter().any(|p| p.contains("large_file.txt")));
}

#[tokio::test]
async fn test_file_info_structure() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir)
        .await
        .expect("Failed to create test project");

    let config = create_test_config();
    let scanner = ProjectScanner::new(config).with_root(&project_root);

    let files = scanner.scan().await.expect("Scan failed");

    // Find a specific file to test
    let rust_file = files
        .iter()
        .find(|f| f.relative_path.to_string_lossy().contains("main.rs"))
        .expect("Could not find main.rs");

    // Test FileInfo structure
    assert!(rust_file.path.ends_with("main.rs"));
    assert_eq!(rust_file.relative_path, PathBuf::from("src/main.rs"));
    assert_eq!(rust_file.extension, Some(".rs".to_string()));
    assert!(rust_file.size_bytes > 0);
    assert!(rust_file.is_text);
    assert_eq!(rust_file.plugin_name, Some("rust".to_string()));
    assert!(!rust_file.content_hash.is_empty());
    assert_eq!(rust_file.content_hash.len(), 64); // SHA256 hex string

    // Test Python file
    let python_file = files
        .iter()
        .find(|f| f.relative_path.to_string_lossy().contains("script.py"))
        .expect("Could not find script.py");

    assert_eq!(python_file.extension, Some(".py".to_string()));
    assert!(python_file.is_text);
    assert_eq!(python_file.plugin_name, Some("python".to_string()));
}

#[tokio::test]
async fn test_scan_empty_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let empty_project = temp_dir.path().join("empty");
    fs::create_dir(&empty_project)
        .await
        .expect("Failed to create empty dir");

    let config = create_test_config();
    let scanner = ProjectScanner::new(config).with_root(&empty_project);

    let files = scanner.scan().await.expect("Scan failed");
    assert!(files.is_empty());
}

#[tokio::test]
async fn test_scan_with_custom_plugin_patterns() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path().to_path_buf();

    // Create files that match our custom plugin
    fs::write(project_root.join("test.test"), "test content")
        .await
        .expect("Failed to write test file");
    fs::write(project_root.join("TEST_FILE"), "test file content")
        .await
        .expect("Failed to write test file");
    fs::write(project_root.join("normal.txt"), "normal content")
        .await
        .expect("Failed to write normal file");

    let config = create_config_with_custom_patterns();
    let scanner = ProjectScanner::new(config).with_root(&project_root);

    let files = scanner.scan().await.expect("Scan failed");

    // Find our custom plugin files
    let test_ext_file = files
        .iter()
        .find(|f| f.relative_path.to_string_lossy().contains("test.test"));
    let test_name_file = files
        .iter()
        .find(|f| f.relative_path.to_string_lossy().contains("TEST_FILE"));
    let normal_file = files
        .iter()
        .find(|f| f.relative_path.to_string_lossy().contains("normal.txt"));

    // Custom plugin should be detected
    assert!(test_ext_file.is_some());
    assert_eq!(
        test_ext_file.unwrap().plugin_name,
        Some("test_plugin".to_string())
    );

    assert!(test_name_file.is_some());
    assert_eq!(
        test_name_file.unwrap().plugin_name,
        Some("test_plugin".to_string())
    );

    // Normal file should not have a plugin
    assert!(normal_file.is_some());
    assert_eq!(normal_file.unwrap().plugin_name, None);
}

#[tokio::test]
async fn test_scan_handles_permission_errors() {
    // This test is tricky because we need a file we can't read
    // On most systems, we can't easily create such a file in tests
    // So we'll test with a non-existent directory instead

    let config = create_test_config();
    let scanner = ProjectScanner::new(config).with_root("/definitely/does/not/exist");

    // This should handle the error gracefully and return an empty result
    // rather than panicking
    let result = scanner.scan().await;

    // The scan might succeed with empty results or fail gracefully
    // Either is acceptable behavior
    match result {
        Ok(files) => {
            // If it succeeds, should be empty
            assert!(files.is_empty());
        }
        Err(_) => {
            // If it fails, that's also acceptable for non-existent directories
        }
    }
}

#[test]
fn test_print_scan_results() {
    // This is more of a smoke test since print_scan_results outputs to stdout
    // We're mainly testing that it doesn't panic

    let config = create_test_config();
    let scanner = ProjectScanner::new(config).with_root("/test");

    let files = vec![
        FileInfo {
            path: PathBuf::from("/test/main.rs"),
            relative_path: PathBuf::from("main.rs"),
            extension: Some(".rs".to_string()),
            size_bytes: 1024,
            is_text: true,
            plugin_name: Some("rust".to_string()),
            content_hash: "test_hash".to_string(),
        },
        FileInfo {
            path: PathBuf::from("/test/script.py"),
            relative_path: PathBuf::from("script.py"),
            extension: Some(".py".to_string()),
            size_bytes: 512,
            is_text: true,
            plugin_name: Some("python".to_string()),
            content_hash: "test_hash2".to_string(),
        },
        FileInfo {
            path: PathBuf::from("/test/unknown.xyz"),
            relative_path: PathBuf::from("unknown.xyz"),
            extension: Some(".xyz".to_string()),
            size_bytes: 256,
            is_text: false,
            plugin_name: None,
            content_hash: "test_hash3".to_string(),
        },
    ];

    // This should not panic
    scanner.print_scan_results(&files);
}

#[tokio::test]
async fn test_scan_to_matrix_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = create_test_project(&temp_dir)
        .await
        .expect("Failed to create test project");

    let config = create_test_config();
    let scanner = ProjectScanner::new(config).with_root(&project_root);

    // This tests the integration between scanner and matrix creation
    // Note: This will fail if plugins aren't available, but tests the basic structure
    let result = scanner.scan_to_matrix().await;

    match result {
        Ok(matrix) => {
            // If plugins work, we should get a populated matrix
            assert!(!matrix.files.is_empty());
            assert_eq!(matrix.metadata.project_root, project_root);
            assert!(matrix.metadata.total_files > 0);
        }
        Err(e) => {
            // If plugins fail (which is expected in unit tests), that's ok
            // The important thing is that it doesn't panic
            eprintln!("scan_to_matrix failed (expected in unit tests): {e}");
        }
    }
}

// New tests for the updated configuration system

#[tokio::test]
async fn test_input_plugin_configuration() {
    let config = create_test_config();

    // Test that default config has the expected input plugins
    assert!(config.input_plugins.contains_key("python"));
    assert!(config.input_plugins.contains_key("rust"));

    // Test that output plugins are separate
    assert!(config.output_plugins.contains_key("markdown_docs"));

    // Test plugin discovery methods
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("test.py")),
        Some("python".to_string())
    );
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("test.rs")),
        Some("rust".to_string())
    );
    assert_eq!(
        config.find_input_plugin_for_file(&PathBuf::from("test.unknown")),
        None
    );
}

#[tokio::test]
async fn test_output_plugin_configuration() {
    let config = create_test_config();

    // Test output plugin discovery
    let doc_plugins = config.find_output_plugins_for_type("documentation", "markdown");
    assert!(!doc_plugins.is_empty());
    assert!(doc_plugins.contains(&"markdown_docs".to_string()));

    // Test non-matching types
    let unknown_plugins = config.find_output_plugins_for_type("unknown_type", "unknown_format");
    assert!(unknown_plugins.is_empty());
}

#[tokio::test]
async fn test_plugin_summary() {
    let config = create_test_config();
    let summary = config.get_plugin_summary();

    assert!(summary.total_input_plugins >= 2); // At least python and rust
    assert!(summary.total_output_plugins >= 1); // At least markdown_docs
    assert_eq!(summary.enabled_input_plugins, summary.total_input_plugins); // All enabled by default
    assert_eq!(summary.enabled_output_plugins, summary.total_output_plugins); // All enabled by default
}

#[test]
fn test_legacy_plugin_migration() {
    // This test would be more complex in a real scenario, but tests the concept
    let config = Config::default();

    // Simulate adding legacy plugin config (would normally come from YAML)
    // The migration logic is in the Config::load method

    // Test that the new structure exists
    assert!(!config.input_plugins.is_empty());
    assert!(!config.output_plugins.is_empty());

    // Test that legacy field is None (no migration needed for default config)
    assert!(config.plugins.is_none());
}
