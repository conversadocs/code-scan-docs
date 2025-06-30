use anyhow::Result;
use log::{info, warn};
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
        info!("Loading configuration from: {}", config_path.display());
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
    info!("Starting project scan...");

    let project_path = path.unwrap_or_else(|| PathBuf::from("."));

    // Create and configure scanner
    let scanner = ProjectScanner::new(config.clone())
        .with_root(&project_path);

    // Perform the scan
    let files = scanner.scan().await?;

    // Print results in a nice format
    scanner.print_scan_results(&files);

    // TODO: Later we'll save to matrix and do LLM analysis
    if !no_llm {
        info!("LLM analysis would happen here (not implemented yet)");
    }

    if include_tests {
        info!("Test file analysis would be included (not implemented yet)");
    }

    // TODO: Handle different output formats and files
    match output {
        crate::cli::args::OutputFormat::Json => {
            info!("JSON output would be generated here");
        }
        crate::cli::args::OutputFormat::Yaml => {
            info!("YAML output would be generated here");
        }
        crate::cli::args::OutputFormat::Pretty => {
            info!("Pretty output already shown above");
        }
    }

    if let Some(output_path) = output_file {
        info!("Results would be saved to: {}", output_path.display());
    }

    Ok(())
}

async fn handle_quality(
    _matrix: Option<PathBuf>,
    _metrics: Vec<crate::cli::args::QualityMetric>,
    _config: &Config,
) -> Result<()> {
    info!("Analyzing code quality...");

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
    info!("Generating documentation...");

    // TODO: Implement documentation generation
    println!("Documentation generation functionality will be implemented here");

    Ok(())
}

async fn handle_plugins(detailed: bool, config: &Config) -> Result<()> {
    info!("Listing available plugins...");

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
    info!("Initializing configuration...");

    let config_path = PathBuf::from(".csdrc.yaml");

    if config_path.exists() && !force {
        return Err(anyhow::anyhow!(
            "Configuration file already exists. Use --force to overwrite."
        ));
    }

    let default_config = Config::default();
    default_config.save(&config_path).await?;

    info!("Created configuration file: {}", config_path.display());

    Ok(())
}
