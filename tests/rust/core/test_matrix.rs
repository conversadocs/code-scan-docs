use std::path::PathBuf;
use tempfile::TempDir;

// Import the modules we're testing
use csd::core::matrix::{
    CodeElement, DependencyType, ElementType, ExternalDependency, FileNode, Import, ImportType,
    ProjectMatrix, Relationship, RelationshipType,
};

// Helper function to create a test FileNode
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
        assert!(matrix.files.is_empty());
        assert!(matrix.relationships.is_empty());
        assert!(matrix.external_dependencies.is_empty());
        assert_eq!(matrix.metadata.csd_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_add_file() {
        let mut matrix = ProjectMatrix::new(PathBuf::from("/test"));
        let file_node = create_test_file_node("src/main.rs", "rust");

        matrix.add_file(file_node.clone());

        assert_eq!(matrix.metadata.total_files, 1);
        assert_eq!(matrix.metadata.total_size_bytes, 1024);
        assert!(matrix.metadata.plugins_used.contains(&"rust".to_string()));
        assert!(matrix.files.contains_key(&PathBuf::from("src/main.rs")));
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
        assert_eq!(matrix.metadata.plugins_used.len(), 2);
        assert!(matrix.metadata.plugins_used.contains(&"rust".to_string()));
        assert!(matrix.metadata.plugins_used.contains(&"python".to_string()));
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
        assert_eq!(loaded_matrix.files.len(), original_matrix.files.len());
        assert_eq!(
            loaded_matrix.relationships.len(),
            original_matrix.relationships.len()
        );

        // Verify specific file exists
        assert!(loaded_matrix
            .files
            .contains_key(&PathBuf::from("src/main.rs")));
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
        };

        assert_eq!(element.name, "test_function");
        assert_eq!(element.line_start, 10);
        assert_eq!(element.line_end, 20);
        assert_eq!(element.complexity_score, Some(5));
        assert_eq!(element.calls.len(), 1);
        assert!(element.calls.contains(&"helper_function".to_string()));
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
}
