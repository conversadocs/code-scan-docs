use anyhow::Result;
use log::{debug, info, warn};
use std::path::PathBuf;

use crate::cli::args::{Args, Command};
use crate::core::scanner::ProjectScanner;
use crate::plugins::manager::PluginManager;
use crate::utils::config::Config;

pub async fn handle_command(args: Args) -> Result<()> {
    // Load configuration
    let config = load_config(&args).await?;

    match args.command {
        Command::Scan {
            path,
            output,
            output_file,
            no_llm,
            include_tests
        } => {
            handle_scan(path, output, output_file, no_llm, include_tests, &config).await
        }
        Command::Quality { matrix, metrics } => {
            handle_quality(matrix, metrics, &config).await
        }
        Command::Docs { matrix, format, output_dir } => {
            handle_docs(matrix, format, output_dir, &config).await
        }
        Command::Plugins { detailed } => {
            handle_plugins(detailed, &config).await
        }
        Command::Init { force } => {
            handle_init(force).await
        }
    }
}

async fn load_config(args: &Args) -> Result<Config> {
    let default_path = PathBuf::from(".csdrc.yaml");
    let config_path = args.config.as_ref().unwrap_or(&default_path);

    if config_path.exists() {
        debug!("Loading configuration from: {}", config_path.display());
        Config::load(config_path).await
    } else {
        warn!("Configuration file not found, using defaults");
        Ok(Config::default())
    }
}

async fn handle_scan(
    path: Option<PathBuf>,
    output: crate::cli::args::OutputFormat,
    output_file: Option<PathBuf>,
    no_llm: bool,
    include_tests: bool,
    config: &Config,
) -> Result<()> {
    info!("Building project matrix...");

    let project_path = path.unwrap_or_else(|| PathBuf::from("."));

    // Create and configure scanner
    let scanner = ProjectScanner::new(config.clone())
        .with_root(&project_path);

    // Perform the scan and build matrix
    let mut matrix = scanner.scan_to_matrix().await?;

    // Print matrix summary
    matrix.print_summary();

    // Save the matrix to cache (this is the primary deliverable)
    let matrix_path = project_path.join(".csd_cache").join("matrix.json");
    matrix.save(&matrix_path).await?;
    info!("Matrix saved to: {}", matrix_path.display());

    // Optional: export matrix to additional formats if requested
    if let Some(output_path) = output_file {
        match output {
            crate::cli::args::OutputFormat::Json => {
                let json_output = serde_json::to_string_pretty(&matrix)?;
                tokio::fs::write(&output_path, json_output).await?;
                info!("Matrix also exported as JSON to: {}", output_path.display());
            }
            crate::cli::args::OutputFormat::Yaml => {
                let yaml_output = serde_yaml::to_string(&matrix)?;
                tokio::fs::write(&output_path, yaml_output).await?;
                info!("Matrix also exported as YAML to: {}", output_path.display());
            }
            crate::cli::args::OutputFormat::Pretty => {
                // For pretty format with output file, save the summary
                let summary = format!("=== Project Matrix Summary ===\nProject: {}\nFiles: {}\nRelationships: {}\nExternal dependencies: {}\nLanguages: {}\n",
                    matrix.metadata.project_root.display(),
                    matrix.metadata.total_files,
                    matrix.relationships.len(),
                    matrix.external_dependencies.len(),
                    matrix.metadata.plugins_used.join(", ")
                );
                tokio::fs::write(&output_path, summary).await?;
                info!("Matrix summary exported to: {}", output_path.display());
            }
        }
    }

    info!("Matrix build complete. Use 'csd quality', 'csd docs', or other commands to analyze the matrix.");

    // Note: --no-llm and --include-tests flags are ignored for scan command
    // Those operations happen in separate commands that use the cached matrix

    Ok(())
}

async fn handle_quality(
    _matrix: Option<PathBuf>,
    _metrics: Vec<crate::cli::args::QualityMetric>,
    _config: &Config,
) -> Result<()> {
    debug!("Analyzing code quality...");

    // TODO: Implement quality analysis
    println!("Quality analysis functionality will be implemented here");

    Ok(())
}

async fn handle_docs(
    _matrix: Option<PathBuf>,
    _format: crate::cli::args::DocFormat,
    _output_dir: Option<PathBuf>,
    _config: &Config,
) -> Result<()> {
    debug!("Generating documentation...");

    // TODO: Implement documentation generation
    println!("Documentation generation functionality will be implemented here");

    Ok(())
}

async fn handle_plugins(detailed: bool, config: &Config) -> Result<()> {
    debug!("Listing available plugins...");

    let plugin_manager = PluginManager::new(config.clone());
    let plugins = plugin_manager.discover_plugins().await?;

    if detailed {
        for plugin in plugins {
            println!("Plugin: {}", plugin.name);
            println!("  Path: {}", plugin.path.display());
            println!("  Extensions: {}", plugin.extensions.join(", "));
            println!("  Filenames: {}", plugin.filenames.join(", "));
            println!("  Source: {:?}", plugin.source);
            println!("  Enabled: {}", plugin.enabled);
            println!();
        }
    } else {
        for plugin in plugins {
            let all_patterns: Vec<String> = plugin.extensions.iter()
                .chain(plugin.filenames.iter())
                .cloned()
                .collect();
            println!("{} - {}", plugin.name, all_patterns.join(", "));
        }
    }

    Ok(())
}

async fn handle_init(force: bool) -> Result<()> {
    debug!("Initializing configuration...");

    let config_path = PathBuf::from(".csdrc.yaml");

    if config_path.exists() && !force {
        return Err(anyhow::anyhow!(
            "Configuration file already exists. Use --force to overwrite."
        ));
    }

    let default_config = Config::default();
    default_config.save(&config_path).await?;

    debug!("Created configuration file: {}", config_path.display());

    Ok(())
}
