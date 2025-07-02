use clap::Parser;
use csd::cli::args::{Args, Command, DocFormat, OutputFormat, QualityMetric};
use std::path::PathBuf;

// Helper function to parse args from a string slice
fn parse_args(args: &[&str]) -> Result<Args, clap::Error> {
    Args::try_parse_from(args)
}

// Helper function to parse args and expect success
fn parse_args_success(args: &[&str]) -> Args {
    parse_args(args).expect("Failed to parse args")
}

#[cfg(test)]
mod basic_parsing_tests {
    use super::*;

    #[test]
    fn test_scan_command_basic() {
        let args = parse_args_success(&["csd", "scan"]);

        match args.command {
            Command::Scan {
                path,
                output,
                output_file,
                no_llm,
                include_tests,
            } => {
                assert!(path.is_none()); // Default: no path specified
                assert!(matches!(output, OutputFormat::Json)); // Default output format
                assert!(output_file.is_none()); // No output file specified
                assert!(!no_llm); // Default: LLM enabled
                assert!(!include_tests); // Default: tests not included
            }
            _ => panic!("Expected Scan command"),
        }

        assert!(!args.verbose); // Default: not verbose
        assert!(args.config.is_none()); // No config specified
        assert!(args.project.is_none()); // No project specified
    }

    #[test]
    fn test_scan_command_with_path() {
        let args = parse_args_success(&["csd", "scan", "/path/to/project"]);

        match args.command {
            Command::Scan { path, .. } => {
                assert_eq!(path, Some(PathBuf::from("/path/to/project")));
            }
            _ => panic!("Expected Scan command"),
        }
    }

    #[test]
    fn test_scan_command_with_all_options() {
        let args = parse_args_success(&[
            "csd",
            "scan",
            "/project",
            "--output",
            "yaml",
            "--output-file",
            "results.yaml",
            "--no-llm",
            "--include-tests",
        ]);

        match args.command {
            Command::Scan {
                path,
                output,
                output_file,
                no_llm,
                include_tests,
            } => {
                assert_eq!(path, Some(PathBuf::from("/project")));
                assert!(matches!(output, OutputFormat::Yaml));
                assert_eq!(output_file, Some(PathBuf::from("results.yaml")));
                assert!(no_llm);
                assert!(include_tests);
            }
            _ => panic!("Expected Scan command"),
        }
    }
}

#[cfg(test)]
mod output_format_tests {
    use super::*;

    #[test]
    fn test_scan_command_output_formats() {
        // Test JSON output format
        let args = parse_args_success(&["csd", "scan", "--output", "json"]);
        match args.command {
            Command::Scan { output, .. } => {
                assert!(matches!(output, OutputFormat::Json));
            }
            _ => panic!("Expected Scan command"),
        }

        // Test YAML output format
        let args = parse_args_success(&["csd", "scan", "--output", "yaml"]);
        match args.command {
            Command::Scan { output, .. } => {
                assert!(matches!(output, OutputFormat::Yaml));
            }
            _ => panic!("Expected Scan command"),
        }

        // Test Pretty output format
        let args = parse_args_success(&["csd", "scan", "--output", "pretty"]);
        match args.command {
            Command::Scan { output, .. } => {
                assert!(matches!(output, OutputFormat::Pretty));
            }
            _ => panic!("Expected Scan command"),
        }
    }
}

#[cfg(test)]
mod quality_command_tests {
    use super::*;

    #[test]
    fn test_quality_command_basic() {
        let args = parse_args_success(&["csd", "quality"]);

        match args.command {
            Command::Quality { matrix, metrics } => {
                assert!(matrix.is_none()); // No matrix file specified
                assert!(metrics.is_empty()); // No specific metrics specified
            }
            _ => panic!("Expected Quality command"),
        }
    }

    #[test]
    fn test_quality_command_with_options() {
        let args = parse_args_success(&[
            "csd",
            "quality",
            "--matrix",
            "/path/to/matrix.json",
            "--metrics",
            "complexity",
            "--metrics",
            "security",
        ]);

        match args.command {
            Command::Quality { matrix, metrics } => {
                assert_eq!(matrix, Some(PathBuf::from("/path/to/matrix.json")));
                assert_eq!(metrics.len(), 2);
                assert!(metrics
                    .iter()
                    .any(|m| matches!(m, QualityMetric::Complexity)));
                assert!(metrics.iter().any(|m| matches!(m, QualityMetric::Security)));
            }
            _ => panic!("Expected Quality command"),
        }
    }

    #[test]
    fn test_quality_metrics_all_types() {
        let args = parse_args_success(&[
            "csd",
            "quality",
            "--metrics",
            "complexity",
            "--metrics",
            "coverage",
            "--metrics",
            "maintainability",
            "--metrics",
            "security",
            "--metrics",
            "performance",
            "--metrics",
            "all",
        ]);

        match args.command {
            Command::Quality { metrics, .. } => {
                assert_eq!(metrics.len(), 6);
                assert!(metrics
                    .iter()
                    .any(|m| matches!(m, QualityMetric::Complexity)));
                assert!(metrics.iter().any(|m| matches!(m, QualityMetric::Coverage)));
                assert!(metrics
                    .iter()
                    .any(|m| matches!(m, QualityMetric::Maintainability)));
                assert!(metrics.iter().any(|m| matches!(m, QualityMetric::Security)));
                assert!(metrics
                    .iter()
                    .any(|m| matches!(m, QualityMetric::Performance)));
                assert!(metrics.iter().any(|m| matches!(m, QualityMetric::All)));
            }
            _ => panic!("Expected Quality command"),
        }
    }
}

#[cfg(test)]
mod docs_command_tests {
    use super::*;

    #[test]
    fn test_docs_command_basic() {
        let args = parse_args_success(&["csd", "docs"]);

        match args.command {
            Command::Docs {
                matrix,
                format,
                output_dir,
            } => {
                assert!(matrix.is_none()); // No matrix file specified
                assert!(matches!(format, DocFormat::Markdown)); // Default format
                assert!(output_dir.is_none()); // No output directory specified
            }
            _ => panic!("Expected Docs command"),
        }
    }

    #[test]
    fn test_docs_command_with_options() {
        let args = parse_args_success(&[
            "csd",
            "docs",
            "--matrix",
            "matrix.json",
            "--format",
            "html",
            "--output-dir",
            "/docs/output",
        ]);

        match args.command {
            Command::Docs {
                matrix,
                format,
                output_dir,
            } => {
                assert_eq!(matrix, Some(PathBuf::from("matrix.json")));
                assert!(matches!(format, DocFormat::Html));
                assert_eq!(output_dir, Some(PathBuf::from("/docs/output")));
            }
            _ => panic!("Expected Docs command"),
        }
    }

    #[test]
    fn test_docs_command_all_formats() {
        // Test Markdown format
        let args = parse_args_success(&["csd", "docs", "--format", "markdown"]);
        match args.command {
            Command::Docs { format, .. } => {
                assert!(matches!(format, DocFormat::Markdown));
            }
            _ => panic!("Expected Docs command"),
        }

        // Test HTML format
        let args = parse_args_success(&["csd", "docs", "--format", "html"]);
        match args.command {
            Command::Docs { format, .. } => {
                assert!(matches!(format, DocFormat::Html));
            }
            _ => panic!("Expected Docs command"),
        }

        // Test PDF format
        let args = parse_args_success(&["csd", "docs", "--format", "pdf"]);
        match args.command {
            Command::Docs { format, .. } => {
                assert!(matches!(format, DocFormat::Pdf));
            }
            _ => panic!("Expected Docs command"),
        }
    }
}

#[cfg(test)]
mod other_commands_tests {
    use super::*;

    #[test]
    fn test_plugins_command_basic() {
        let args = parse_args_success(&["csd", "plugins"]);

        match args.command {
            Command::Plugins { detailed } => {
                assert!(!detailed); // Default: not detailed
            }
            _ => panic!("Expected Plugins command"),
        }
    }

    #[test]
    fn test_plugins_command_detailed() {
        let args = parse_args_success(&["csd", "plugins", "--detailed"]);

        match args.command {
            Command::Plugins { detailed } => {
                assert!(detailed);
            }
            _ => panic!("Expected Plugins command"),
        }
    }

    #[test]
    fn test_init_command_basic() {
        let args = parse_args_success(&["csd", "init"]);

        match args.command {
            Command::Init { force } => {
                assert!(!force); // Default: not forced
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_init_command_force() {
        let args = parse_args_success(&["csd", "init", "--force"]);

        match args.command {
            Command::Init { force } => {
                assert!(force);
            }
            _ => panic!("Expected Init command"),
        }
    }
}

#[cfg(test)]
mod global_flags_tests {
    use super::*;

    #[test]
    fn test_global_verbose_flag() {
        let args = parse_args_success(&["csd", "--verbose", "scan"]);
        assert!(args.verbose);

        let args = parse_args_success(&["csd", "-v", "scan"]);
        assert!(args.verbose);

        // Test verbose flag can come after subcommand too
        let args = parse_args_success(&["csd", "scan", "--verbose"]);
        assert!(args.verbose);
    }

    #[test]
    fn test_global_config_flag() {
        let args = parse_args_success(&["csd", "--config", "/path/to/config.yaml", "scan"]);
        assert_eq!(args.config, Some(PathBuf::from("/path/to/config.yaml")));

        let args = parse_args_success(&["csd", "-c", "config.yaml", "scan"]);
        assert_eq!(args.config, Some(PathBuf::from("config.yaml")));
    }

    #[test]
    fn test_global_project_flag() {
        let args = parse_args_success(&["csd", "--project", "/project/root", "scan"]);
        assert_eq!(args.project, Some(PathBuf::from("/project/root")));

        let args = parse_args_success(&["csd", "-p", "/root", "scan"]);
        assert_eq!(args.project, Some(PathBuf::from("/root")));
    }

    #[test]
    fn test_global_flags_combination() {
        let args = parse_args_success(&[
            "csd",
            "--verbose",
            "--config",
            "custom.yaml",
            "--project",
            "/my/project",
            "scan",
            "--output",
            "yaml",
        ]);

        assert!(args.verbose);
        assert_eq!(args.config, Some(PathBuf::from("custom.yaml")));
        assert_eq!(args.project, Some(PathBuf::from("/my/project")));

        match args.command {
            Command::Scan { output, .. } => {
                assert!(matches!(output, OutputFormat::Yaml));
            }
            _ => panic!("Expected Scan command"),
        }
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_invalid_command() {
        let result = parse_args(&["csd", "invalid-command"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_output_format() {
        let result = parse_args(&["csd", "scan", "--output", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_doc_format() {
        let result = parse_args(&["csd", "docs", "--format", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_quality_metric() {
        let result = parse_args(&["csd", "quality", "--metrics", "invalid"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_command() {
        let result = parse_args(&["csd"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_help_flag() {
        // These should fail because --help causes clap to exit
        let result = parse_args(&["csd", "--help"]);
        assert!(result.is_err());

        let result = parse_args(&["csd", "scan", "--help"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_version_flag() {
        // This should fail because --version causes clap to exit
        let result = parse_args(&["csd", "--version"]);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod path_and_format_tests {
    use super::*;

    #[test]
    fn test_short_flags() {
        let args = parse_args_success(&["csd", "scan", "-o", "yaml", "-f", "output.yaml"]);

        match args.command {
            Command::Scan {
                output,
                output_file,
                ..
            } => {
                assert!(matches!(output, OutputFormat::Yaml));
                assert_eq!(output_file, Some(PathBuf::from("output.yaml")));
            }
            _ => panic!("Expected Scan command"),
        }
    }

    #[test]
    fn test_path_handling() {
        // Test relative paths
        let args = parse_args_success(&["csd", "scan", "relative/path"]);
        match args.command {
            Command::Scan { path, .. } => {
                assert_eq!(path, Some(PathBuf::from("relative/path")));
            }
            _ => panic!("Expected Scan command"),
        }

        // Test absolute paths
        let args = parse_args_success(&["csd", "scan", "/absolute/path"]);
        match args.command {
            Command::Scan { path, .. } => {
                assert_eq!(path, Some(PathBuf::from("/absolute/path")));
            }
            _ => panic!("Expected Scan command"),
        }

        // Test current directory
        let args = parse_args_success(&["csd", "scan", "."]);
        match args.command {
            Command::Scan { path, .. } => {
                assert_eq!(path, Some(PathBuf::from(".")));
            }
            _ => panic!("Expected Scan command"),
        }
    }
}

#[cfg(test)]
mod comprehensive_tests {
    use super::*;

    #[test]
    fn test_scan_command_comprehensive() {
        // Test all combinations of scan command options since this is the primary implemented command

        // Basic scan
        let args = parse_args_success(&["csd", "scan"]);
        match args.command {
            Command::Scan {
                path,
                output,
                output_file,
                no_llm,
                include_tests,
            } => {
                assert!(path.is_none());
                assert!(matches!(output, OutputFormat::Json));
                assert!(output_file.is_none());
                assert!(!no_llm);
                assert!(!include_tests);
            }
            _ => panic!("Expected Scan command"),
        }

        // Scan with path
        let args = parse_args_success(&["csd", "scan", "/my/project"]);
        match args.command {
            Command::Scan { path, .. } => {
                assert_eq!(path, Some(PathBuf::from("/my/project")));
            }
            _ => panic!("Expected Scan command"),
        }

        // Scan with output formats
        for (format_str, expected_format) in [
            ("json", OutputFormat::Json),
            ("yaml", OutputFormat::Yaml),
            ("pretty", OutputFormat::Pretty),
        ] {
            let args = parse_args_success(&["csd", "scan", "--output", format_str]);
            match args.command {
                Command::Scan { output, .. } => {
                    assert!(
                        std::mem::discriminant(&output) == std::mem::discriminant(&expected_format)
                    );
                }
                _ => panic!("Expected Scan command"),
            }
        }

        // Scan with flags
        let args = parse_args_success(&["csd", "scan", "--no-llm", "--include-tests"]);
        match args.command {
            Command::Scan {
                no_llm,
                include_tests,
                ..
            } => {
                assert!(no_llm);
                assert!(include_tests);
            }
            _ => panic!("Expected Scan command"),
        }
    }

    #[test]
    fn test_complex_quality_scenario() {
        let args = parse_args_success(&[
            "csd",
            "--config",
            "custom-config.yaml",
            "quality",
            "--matrix",
            "analysis-matrix.json",
            "--metrics",
            "complexity",
            "--metrics",
            "security",
            "--metrics",
            "maintainability",
        ]);

        assert_eq!(args.config, Some(PathBuf::from("custom-config.yaml")));

        match args.command {
            Command::Quality { matrix, metrics } => {
                assert_eq!(matrix, Some(PathBuf::from("analysis-matrix.json")));
                assert_eq!(metrics.len(), 3);
                assert!(metrics
                    .iter()
                    .any(|m| matches!(m, QualityMetric::Complexity)));
                assert!(metrics.iter().any(|m| matches!(m, QualityMetric::Security)));
                assert!(metrics
                    .iter()
                    .any(|m| matches!(m, QualityMetric::Maintainability)));
            }
            _ => panic!("Expected Quality command"),
        }
    }

    #[test]
    fn test_debug_and_clone_traits() {
        let args = parse_args_success(&["csd", "scan"]);

        // Test Debug trait
        let debug_str = format!("{args:?}");
        assert!(debug_str.contains("Args"));

        // Test that the command enum variants implement Debug
        let debug_str = format!("{:?}", args.command);
        assert!(debug_str.contains("Scan"));

        // Test Clone trait
        let _cloned_args = args.clone();

        // Test that enums also have Debug/Clone
        let output_format = OutputFormat::Json;
        let _cloned_format = output_format.clone();
        let debug_output = format!("{output_format:?}");
        assert!(debug_output.contains("Json"));

        let quality_metric = QualityMetric::Complexity;
        let _cloned_metric = quality_metric.clone();
        let debug_metric = format!("{quality_metric:?}");
        assert!(debug_metric.contains("Complexity"));
    }
}
