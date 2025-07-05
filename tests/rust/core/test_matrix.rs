use std::path::PathBuf;
use tempfile::TempDir;

// Import the modules we're testing
use csd::core::matrix::{
    estimate_code_tokens, estimate_tokens, CodeElement, DependencyType, ElementType,
    EntrypointInfo, ExternalDependency, FileNode, Import, ImportType, ProjectMatrix, ProjectType,
    Relationship, RelationshipType, TokenInfo,
};

// Helper function to create a test FileNode with token information
pub fn create_test_file_node(path: &str, plugin: &str) -> FileNode {
    FileNode {
        path: PathBuf::from(path),
        relative_path: PathBuf::from(path),
        hash: "test_hash_123".to_string(),
        size_bytes: 1024,
        plugin: plugin.to_string(),
        language: Some(plugin.to_string()),
        is_text: true,
        elements: vec![],
        imports: vec![],
        exports: vec![],
        file_summary: Some("Test file summary".to_string()),
        token_info: TokenInfo {
            total_tokens: 256,
            code_tokens: 200,
            documentation_tokens: 40,
            comment_tokens: 16,
        },
    }
}

// Helper function to create a test Relationship
pub fn create_test_relationship(from: &str, to: &str) -> Relationship {
    Relationship {
        from_file: PathBuf::from(from),
        to_file: PathBuf::from(to),
        relationship_type: RelationshipType::Import,
        details: "test import".to_string(),
        line_number: Some(10),
        strength: 0.8,
    }
}

#[cfg(test)]
mod matrix_creation_tests {
    use super::*;

    #[test]
    fn test_project_matrix_creation() {
        let project_root = PathBuf::from("/test/project");
        let matrix = ProjectMatrix::new(project_root.clone());

        assert_eq!(matrix.metadata.project_root, project_root);
        assert_eq!(matrix.metadata.total_files, 0);
        assert_eq!(matrix.metadata.total_size_bytes, 0);
        assert_eq!(matrix.metadata.total_tokens, 0);
        assert!(matrix.files.is_empty());
        assert!(matrix.relationships.is_empty());
        assert!(matrix.external_dependencies.is_empty());
        assert_eq!(matrix.metadata.csd_version, env!("CARGO_PKG_VERSION"));

        // Check new project info fields
        assert!(matrix.project_info.entrypoints.is_empty());
        assert!(matches!(
            matrix.project_info.project_type,
            ProjectType::Unknown
        ));
        assert_eq!(matrix.project_info.main_language, "");
        assert_eq!(matrix.project_info.token_summary.total_tokens, 0);
    }

    #[test]
    fn test_add_file() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));
        let file_node = create_test_file_node("src/main.rs", "rust");

        matrix.add_file(file_node.clone());

        assert_eq!(matrix.metadata.total_files, 1);
        assert_eq!(matrix.metadata.total_size_bytes, 1024);
        assert_eq!(matrix.metadata.total_tokens, 256);
        assert!(matrix.metadata.plugins_used.contains(&"rust".to_string()));
        assert!(matrix.files.contains_key(&PathBuf::from("src/main.rs")));

        // Check token tracking
        assert_eq!(matrix.project_info.token_summary.total_tokens, 256);
        assert_eq!(matrix.project_info.token_summary.code_tokens, 200);
        assert_eq!(matrix.project_info.token_summary.documentation_tokens, 40);
    }

    #[test]
    fn test_add_multiple_files_different_plugins() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));
        let rust_file = create_test_file_node("src/main.rs", "rust");
        let python_file = create_test_file_node("script.py", "python");

        matrix.add_file(rust_file);
        matrix.add_file(python_file);

        assert_eq!(matrix.metadata.total_files, 2);
        assert_eq!(matrix.metadata.total_size_bytes, 2048);
        assert_eq!(matrix.metadata.total_tokens, 512);
        assert_eq!(matrix.metadata.plugins_used.len(), 2);
        assert!(matrix.metadata.plugins_used.contains(&"rust".to_string()));
        assert!(matrix.metadata.plugins_used.contains(&"python".to_string()));
    }

    #[test]
    fn test_finalize_detects_entrypoints() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));

        // Add main.rs as an entrypoint
        let main_file = create_test_file_node("src/main.rs", "rust");
        matrix.add_file(main_file);

        // Add lib.rs as another entrypoint
        let lib_file = create_test_file_node("src/lib.rs", "rust");
        matrix.add_file(lib_file);

        // Add a regular file
        let util_file = create_test_file_node("src/utils.rs", "rust");
        matrix.add_file(util_file);

        // Finalize to detect entrypoints
        matrix.finalize();

        // Check entrypoints were detected
        assert_eq!(matrix.project_info.entrypoints.len(), 2);

        // Check that main.rs was detected
        let main_entry = matrix
            .project_info
            .entrypoints
            .iter()
            .find(|e| e.file_path == PathBuf::from("src/main.rs"))
            .expect("main.rs should be detected as entrypoint");
        assert_eq!(main_entry.entrypoint_type, "cli");
        assert_eq!(main_entry.confidence, 1.0);

        // Check that lib.rs was detected
        let lib_entry = matrix
            .project_info
            .entrypoints
            .iter()
            .find(|e| e.file_path == PathBuf::from("src/lib.rs"))
            .expect("lib.rs should be detected as entrypoint");
        assert_eq!(lib_entry.entrypoint_type, "lib");
        assert_eq!(lib_entry.confidence, 1.0);

        // Check project type
        assert!(matches!(
            matrix.project_info.project_type,
            ProjectType::Mixed
        ));
        assert_eq!(matrix.project_info.main_language, "rust");
    }

    #[test]
    fn test_add_relationship() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));
        let relationship = create_test_relationship("src/main.rs", "src/lib.rs");

        matrix.add_relationship(relationship.clone());

        assert_eq!(matrix.relationships.len(), 1);
        let added_rel = &matrix.relationships[0];
        assert_eq!(added_rel.from_file, PathBuf::from("src/main.rs"));
        assert_eq!(added_rel.to_file, PathBuf::from("src/lib.rs"));
        assert_eq!(added_rel.strength, 0.8);
    }

    #[test]
    fn test_add_external_dependency() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));
        let dependency = ExternalDependency {
            name: "serde".to_string(),
            version: Some("1.0.0".to_string()),
            ecosystem: "cargo".to_string(),
            dependency_type: DependencyType::Runtime,
            source_file: PathBuf::from("Cargo.toml"),
        };

        matrix.add_external_dependency(dependency.clone());

        assert_eq!(matrix.external_dependencies.len(), 1);
        let added_dep = &matrix.external_dependencies[0];
        assert_eq!(added_dep.name, "serde");
        assert_eq!(added_dep.version, Some("1.0.0".to_string()));
        assert_eq!(added_dep.ecosystem, "cargo");
    }
}

#[cfg(test)]
mod token_management_tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        let text = "Hello, world!";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
        assert_eq!(tokens, 4); // ~13 chars / 4 = ~3.25, rounded up to 4

        let code = "fn main() { println!(\"Hello\"); }";
        let code_tokens = estimate_code_tokens(code);
        assert!(code_tokens > 0);
        // Should account for delimiters and tokens
    }

    #[test]
    fn test_get_files_by_token_count() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));

        // Create files with different token counts
        let mut large_file = create_test_file_node("large.rs", "rust");
        large_file.token_info.total_tokens = 1000;

        let mut medium_file = create_test_file_node("medium.rs", "rust");
        medium_file.token_info.total_tokens = 500;

        let mut small_file = create_test_file_node("small.rs", "rust");
        small_file.token_info.total_tokens = 100;

        matrix.add_file(large_file);
        matrix.add_file(medium_file);
        matrix.add_file(small_file);

        let sorted_files = matrix.get_files_by_token_count();

        // Should be sorted by token count descending
        assert_eq!(sorted_files.len(), 3);
        assert_eq!(sorted_files[0].1.token_info.total_tokens, 1000);
        assert_eq!(sorted_files[1].1.token_info.total_tokens, 500);
        assert_eq!(sorted_files[2].1.token_info.total_tokens, 100);
    }

    #[test]
    fn test_token_budget_info() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));

        // Create files with specific token counts
        let mut file1 = create_test_file_node("file1.rs", "rust");
        file1.token_info.total_tokens = 400;

        let mut file2 = create_test_file_node("file2.rs", "rust");
        file2.token_info.total_tokens = 300;

        let mut file3 = create_test_file_node("file3.rs", "rust");
        file3.token_info.total_tokens = 500;

        matrix.add_file(file1);
        matrix.add_file(file2);
        matrix.add_file(file3);

        // Get budget info with limit of 800 tokens
        let budget_info = matrix.get_token_budget_info(800);

        assert_eq!(budget_info.max_tokens, 800);
        // Files are sorted by token count descending, so file3 (500) + file2 (300) = 800
        assert_eq!(budget_info.used_tokens, 800);
        assert_eq!(budget_info.remaining_tokens, 0);
        assert_eq!(budget_info.included_files.len(), 2);
        assert_eq!(budget_info.excluded_files.len(), 1);

        // The 400-token file should be excluded (since 500 + 300 = 800 exactly)
        assert!(budget_info
            .excluded_files
            .contains(&PathBuf::from("file1.rs")));

        // The included files should be file3 and file2
        assert!(budget_info
            .included_files
            .contains(&PathBuf::from("file3.rs")));
        assert!(budget_info
            .included_files
            .contains(&PathBuf::from("file2.rs")));
    }
}

#[cfg(test)]
mod matrix_queries_tests {
    use super::*;

    #[test]
    fn test_get_files_by_plugin() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));
        let rust_file1 = create_test_file_node("src/main.rs", "rust");
        let rust_file2 = create_test_file_node("src/lib.rs", "rust");
        let python_file = create_test_file_node("script.py", "python");

        matrix.add_file(rust_file1);
        matrix.add_file(rust_file2);
        matrix.add_file(python_file);

        let rust_files = matrix.get_files_by_plugin("rust");
        let python_files = matrix.get_files_by_plugin("python");
        let missing_files = matrix.get_files_by_plugin("javascript");

        assert_eq!(rust_files.len(), 2);
        assert_eq!(python_files.len(), 1);
        assert_eq!(missing_files.len(), 0);
    }

    #[test]
    fn test_find_dependencies_and_dependents() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));

        // Create files
        let main_file = create_test_file_node("src/main.rs", "rust");
        let lib_file = create_test_file_node("src/lib.rs", "rust");
        let utils_file = create_test_file_node("src/utils.rs", "rust");

        // Add files to matrix
        matrix.add_file(main_file);
        matrix.add_file(lib_file);
        matrix.add_file(utils_file);

        // Create relationships: main.rs -> lib.rs -> utils.rs
        let rel1 = create_test_relationship("src/main.rs", "src/lib.rs");
        let rel2 = create_test_relationship("src/lib.rs", "src/utils.rs");

        matrix.add_relationship(rel1);
        matrix.add_relationship(rel2);

        // Test dependencies (what main.rs depends on)
        let main_deps = matrix.find_dependencies(&PathBuf::from("src/main.rs"));
        assert_eq!(main_deps.len(), 1);
        assert_eq!(main_deps[0].path, PathBuf::from("src/lib.rs"));

        // Test dependents (what depends on lib.rs)
        let lib_dependents = matrix.find_dependents(&PathBuf::from("src/lib.rs"));
        assert_eq!(lib_dependents.len(), 1);
        assert_eq!(lib_dependents[0].path, PathBuf::from("src/main.rs"));

        // Test file with no dependencies
        let utils_deps = matrix.find_dependencies(&PathBuf::from("src/utils.rs"));
        assert_eq!(utils_deps.len(), 0);

        // Test file with no dependents
        let utils_dependents = matrix.find_dependents(&PathBuf::from("src/utils.rs"));
        assert_eq!(utils_dependents.len(), 1);
        assert_eq!(utils_dependents[0].path, PathBuf::from("src/lib.rs"));
    }

    #[test]
    fn test_calculate_metrics() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));

        // Create a small project structure
        let files = vec![
            create_test_file_node("src/main.rs", "rust"),
            create_test_file_node("src/lib.rs", "rust"),
            create_test_file_node("src/utils.rs", "rust"),
            create_test_file_node("script.py", "python"),
        ];

        for file in files {
            matrix.add_file(file);
        }

        // Add some relationships to make lib.rs highly coupled
        let relationships = vec![
            create_test_relationship("src/main.rs", "src/lib.rs"),
            create_test_relationship("script.py", "src/lib.rs"),
            create_test_relationship("src/utils.rs", "src/lib.rs"),
        ];

        for rel in relationships {
            matrix.add_relationship(rel);
        }

        let metrics = matrix.calculate_metrics();

        assert_eq!(metrics.total_files, 4);
        assert_eq!(metrics.total_relationships, 3);
        assert_eq!(metrics.total_tokens, 1024); // 4 files * 256 tokens each
        assert_eq!(metrics.languages.len(), 2);
        assert!(metrics.languages.contains(&"rust".to_string()));
        assert!(metrics.languages.contains(&"python".to_string()));

        // lib.rs should be the most highly coupled (3 incoming dependencies)
        assert!(!metrics.highly_coupled_files.is_empty());
        assert_eq!(
            metrics.highly_coupled_files[0].0,
            PathBuf::from("src/lib.rs")
        );
        assert_eq!(metrics.highly_coupled_files[0].1, 3); // 3 incoming edges
    }
}

#[cfg(test)]
mod matrix_persistence_tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_load_matrix() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let matrix_path = temp_dir.path().join("test_matrix.json");

        // Create and populate a matrix
        let mut original_matrix = ProjectMatrix::new(PathBuf::from("/test/project"));
        let file_node = create_test_file_node("src/main.rs", "rust");
        let relationship = create_test_relationship("src/main.rs", "src/lib.rs");

        original_matrix.add_file(file_node);
        original_matrix.add_relationship(relationship);

        // Finalize to ensure all fields are populated
        original_matrix.finalize();

        // Save the matrix
        original_matrix
            .save(&matrix_path)
            .await
            .expect("Failed to save matrix");

        // Verify the file was created
        assert!(matrix_path.exists());

        // Load the matrix
        let loaded_matrix = ProjectMatrix::load(&matrix_path)
            .await
            .expect("Failed to load matrix");

        // Verify the loaded matrix matches the original
        assert_eq!(
            loaded_matrix.metadata.project_root,
            original_matrix.metadata.project_root
        );
        assert_eq!(
            loaded_matrix.metadata.total_files,
            original_matrix.metadata.total_files
        );
        assert_eq!(
            loaded_matrix.metadata.total_tokens,
            original_matrix.metadata.total_tokens
        );
        assert_eq!(loaded_matrix.files.len(), original_matrix.files.len());
        assert_eq!(
            loaded_matrix.relationships.len(),
            original_matrix.relationships.len()
        );

        // Verify specific file exists
        assert!(loaded_matrix
            .files
            .contains_key(&PathBuf::from("src/main.rs")));

        // Verify token info is preserved
        let loaded_file = loaded_matrix
            .files
            .get(&PathBuf::from("src/main.rs"))
            .unwrap();
        assert_eq!(loaded_file.token_info.total_tokens, 256);
    }

    #[tokio::test]
    async fn test_load_subset() {
        // Create a temporary directory for testing
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let matrix_path = temp_dir.path().join("test_matrix.json");

        // Create and populate a full matrix
        let mut original_matrix = ProjectMatrix::new(PathBuf::from("/test/project"));

        let files = vec![
            create_test_file_node("src/main.rs", "rust"),
            create_test_file_node("src/lib.rs", "rust"),
            create_test_file_node("src/utils.rs", "rust"),
        ];

        for file in files {
            original_matrix.add_file(file);
        }

        // Add relationships
        original_matrix.add_relationship(create_test_relationship("src/main.rs", "src/lib.rs"));
        original_matrix.add_relationship(create_test_relationship("src/lib.rs", "src/utils.rs"));

        original_matrix.finalize();

        // Save the matrix
        original_matrix
            .save(&matrix_path)
            .await
            .expect("Failed to save matrix");

        // Load a subset containing only main.rs and lib.rs
        let subset_files = vec![PathBuf::from("src/main.rs"), PathBuf::from("src/lib.rs")];

        let subset_matrix = ProjectMatrix::load_subset(&matrix_path, &subset_files)
            .await
            .expect("Failed to load subset");

        // Verify subset contains only requested files
        assert_eq!(subset_matrix.files.len(), 2);
        assert!(subset_matrix
            .files
            .contains_key(&PathBuf::from("src/main.rs")));
        assert!(subset_matrix
            .files
            .contains_key(&PathBuf::from("src/lib.rs")));
        assert!(!subset_matrix
            .files
            .contains_key(&PathBuf::from("src/utils.rs")));

        // Verify only relationships between included files are preserved
        assert_eq!(subset_matrix.relationships.len(), 1);
        assert_eq!(
            subset_matrix.relationships[0].from_file,
            PathBuf::from("src/main.rs")
        );
        assert_eq!(
            subset_matrix.relationships[0].to_file,
            PathBuf::from("src/lib.rs")
        );
    }
}

#[cfg(test)]
mod matrix_summary_tests {
    use super::*;

    #[test]
    fn test_print_summary() {
        // This is more of a smoke test since print_summary outputs to stdout
        // We're mainly testing that it doesn't panic

        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));

        let files = vec![
            create_test_file_node("src/main.rs", "rust"),
            create_test_file_node("script.py", "python"),
        ];

        for file in files {
            matrix.add_file(file);
        }

        let relationship = create_test_relationship("src/main.rs", "script.py");
        matrix.add_relationship(relationship);

        let dependency = ExternalDependency {
            name: "serde".to_string(),
            version: Some("1.0.0".to_string()),
            ecosystem: "cargo".to_string(),
            dependency_type: DependencyType::Runtime,
            source_file: PathBuf::from("Cargo.toml"),
        };
        matrix.add_external_dependency(dependency);

        // Finalize to calculate token averages
        matrix.finalize();

        // This should not panic
        matrix.print_summary();
    }
}

#[cfg(test)]
mod data_structure_tests {
    use super::*;

    #[test]
    fn test_code_element_creation() {
        let element = CodeElement {
            element_type: ElementType::Function,
            name: "test_function".to_string(),
            signature: Some("fn test_function() -> bool".to_string()),
            line_start: 10,
            line_end: 20,
            summary: Some("A test function".to_string()),
            complexity_score: Some(5),
            calls: vec!["helper_function".to_string()],
            metadata: serde_json::json!({
                "is_async": false,
                "visibility": "public"
            }),
            tokens: 150,
        };

        assert_eq!(element.name, "test_function");
        assert_eq!(element.line_start, 10);
        assert_eq!(element.line_end, 20);
        assert_eq!(element.complexity_score, Some(5));
        assert_eq!(element.calls.len(), 1);
        assert!(element.calls.contains(&"helper_function".to_string()));
        assert_eq!(element.tokens, 150);
    }

    #[test]
    fn test_import_creation() {
        let import = Import {
            module: "std::collections".to_string(),
            items: vec!["HashMap".to_string(), "HashSet".to_string()],
            alias: Some("collections".to_string()),
            line_number: 5,
            import_type: ImportType::Standard,
        };

        assert_eq!(import.module, "std::collections");
        assert_eq!(import.items.len(), 2);
        assert!(import.items.contains(&"HashMap".to_string()));
        assert!(import.items.contains(&"HashSet".to_string()));
        assert_eq!(import.alias, Some("collections".to_string()));
        assert_eq!(import.line_number, 5);
        assert!(matches!(import.import_type, ImportType::Standard));
    }

    #[test]
    fn test_relationship_creation() {
        let relationship = Relationship {
            from_file: PathBuf::from("src/main.rs"),
            to_file: PathBuf::from("src/lib.rs"),
            relationship_type: RelationshipType::Import,
            details: "imports lib module".to_string(),
            line_number: Some(15),
            strength: 0.8,
        };

        assert_eq!(relationship.from_file, PathBuf::from("src/main.rs"));
        assert_eq!(relationship.to_file, PathBuf::from("src/lib.rs"));
        assert!(matches!(
            relationship.relationship_type,
            RelationshipType::Import
        ));
        assert_eq!(relationship.details, "imports lib module");
        assert_eq!(relationship.line_number, Some(15));
        assert_eq!(relationship.strength, 0.8);
    }

    #[test]
    fn test_external_dependency_creation() {
        let dependency = ExternalDependency {
            name: "tokio".to_string(),
            version: Some("1.0.0".to_string()),
            ecosystem: "cargo".to_string(),
            dependency_type: DependencyType::Runtime,
            source_file: PathBuf::from("Cargo.toml"),
        };

        assert_eq!(dependency.name, "tokio");
        assert_eq!(dependency.version, Some("1.0.0".to_string()));
        assert_eq!(dependency.ecosystem, "cargo");
        assert!(matches!(
            dependency.dependency_type,
            DependencyType::Runtime
        ));
        assert_eq!(dependency.source_file, PathBuf::from("Cargo.toml"));
    }

    #[test]
    fn test_file_node_creation() {
        let file_node = FileNode {
            path: PathBuf::from("/project/src/main.rs"),
            relative_path: PathBuf::from("src/main.rs"),
            hash: "abc123def456".to_string(),
            size_bytes: 2048,
            plugin: "rust".to_string(),
            language: Some("rust".to_string()),
            is_text: true,
            elements: vec![],
            imports: vec![],
            exports: vec!["main".to_string()],
            file_summary: Some("Main application file".to_string()),
            token_info: TokenInfo {
                total_tokens: 512,
                code_tokens: 400,
                documentation_tokens: 80,
                comment_tokens: 32,
            },
        };

        assert_eq!(file_node.path, PathBuf::from("/project/src/main.rs"));
        assert_eq!(file_node.relative_path, PathBuf::from("src/main.rs"));
        assert_eq!(file_node.hash, "abc123def456");
        assert_eq!(file_node.size_bytes, 2048);
        assert_eq!(file_node.plugin, "rust");
        assert_eq!(file_node.language, Some("rust".to_string()));
        assert!(file_node.is_text);
        assert_eq!(file_node.exports, vec!["main".to_string()]);
        assert_eq!(
            file_node.file_summary,
            Some("Main application file".to_string())
        );
        assert_eq!(file_node.token_info.total_tokens, 512);
    }

    #[test]
    fn test_token_info_creation() {
        let token_info = TokenInfo {
            total_tokens: 1000,
            code_tokens: 800,
            documentation_tokens: 150,
            comment_tokens: 50,
        };

        assert_eq!(token_info.total_tokens, 1000);
        assert_eq!(token_info.code_tokens, 800);
        assert_eq!(token_info.documentation_tokens, 150);
        assert_eq!(token_info.comment_tokens, 50);
    }

    #[test]
    fn test_entrypoint_info_creation() {
        let entrypoint = EntrypointInfo {
            file_path: PathBuf::from("src/main.rs"),
            entrypoint_type: "cli".to_string(),
            confidence: 0.95,
            reason: "Standard Rust binary entrypoint".to_string(),
        };

        assert_eq!(entrypoint.file_path, PathBuf::from("src/main.rs"));
        assert_eq!(entrypoint.entrypoint_type, "cli");
        assert_eq!(entrypoint.confidence, 0.95);
        assert_eq!(entrypoint.reason, "Standard Rust binary entrypoint");
    }
}

#[cfg(test)]
mod enum_variant_tests {
    use super::*;

    #[test]
    fn test_element_type_variants() {
        let variants = [
            ElementType::Function,
            ElementType::Method,
            ElementType::Class,
            ElementType::Struct,
            ElementType::Enum,
            ElementType::Interface,
            ElementType::Module,
            ElementType::Variable,
            ElementType::Constant,
            ElementType::Type,
        ];

        // Test that all variants can be created and compared
        assert_eq!(variants.len(), 10);
        assert!(variants.contains(&ElementType::Function));
        assert!(variants.contains(&ElementType::Struct));
    }

    #[test]
    fn test_import_type_variants() {
        let variants = [
            ImportType::Standard,
            ImportType::ThirdParty,
            ImportType::Local,
            ImportType::Relative,
        ];

        assert_eq!(variants.len(), 4);
        assert!(variants.contains(&ImportType::Standard));
        assert!(variants.contains(&ImportType::Local));
    }

    #[test]
    fn test_relationship_type_variants() {
        let variants = [
            RelationshipType::Import,
            RelationshipType::Call,
            RelationshipType::Inheritance,
            RelationshipType::Configuration,
            RelationshipType::Test,
            RelationshipType::Documentation,
            RelationshipType::Build,
        ];

        assert_eq!(variants.len(), 7);
        assert!(variants.contains(&RelationshipType::Import));
        assert!(variants.contains(&RelationshipType::Call));
    }

    #[test]
    fn test_dependency_type_variants() {
        let variants = [
            DependencyType::Runtime,
            DependencyType::Development,
            DependencyType::Build,
            DependencyType::Optional,
        ];

        assert_eq!(variants.len(), 4);
        assert!(variants.contains(&DependencyType::Runtime));
        assert!(variants.contains(&DependencyType::Development));
    }

    #[test]
    fn test_project_type_variants() {
        let variants = [
            ProjectType::Binary,
            ProjectType::Library,
            ProjectType::WebApplication,
            ProjectType::Mixed,
            ProjectType::Unknown,
        ];

        assert_eq!(variants.len(), 5);
        assert!(matches!(variants[0], ProjectType::Binary));
        assert!(matches!(variants[1], ProjectType::Library));
        assert!(matches!(variants[2], ProjectType::WebApplication));
        assert!(matches!(variants[3], ProjectType::Mixed));
        assert!(matches!(variants[4], ProjectType::Unknown));
    }
}
