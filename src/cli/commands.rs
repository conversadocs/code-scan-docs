use anyhow::Result;
use log::{debug, info, warn};
use std::path::PathBuf;

use crate::cli::args::{Args, Command};
use crate::core::scanner::ProjectScanner;
use crate::plugins::interface::{OutputPluginInput, OutputPluginInterface}; // Added missing imports
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
            include_tests,
        } => handle_scan(path, output, output_file, no_llm, include_tests, &config).await,
        Command::Quality { matrix, metrics } => handle_quality(matrix, metrics, &config).await,
        Command::Docs {
            matrix,
            format,
            output_dir,
        } => handle_docs(matrix, format, output_dir, &config).await,
        Command::Plugins { detailed } => handle_plugins(detailed, &config).await,
        Command::Init { force } => handle_init(force).await,
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
    _no_llm: bool,
    _include_tests: bool,
    config: &Config,
) -> Result<()> {
    info!("Building project matrix...");

    let project_path = path.unwrap_or_else(|| PathBuf::from("."));

    // Create and configure scanner
    let scanner = ProjectScanner::new(config.clone()).with_root(&project_path);

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

    Ok(())
}

async fn handle_quality(
    matrix: Option<PathBuf>,
    _metrics: Vec<crate::cli::args::QualityMetric>,
    config: &Config,
) -> Result<()> {
    debug!("Analyzing code quality...");

    let matrix_path = matrix.unwrap_or_else(|| PathBuf::from(".csd_cache/matrix.json"));

    if !matrix_path.exists() {
        return Err(anyhow::anyhow!(
            "Matrix file not found: {}. Run 'csd scan' first.",
            matrix_path.display()
        ));
    }

    // Find quality analysis output plugins
    let quality_plugins = config.find_output_plugins_for_type("quality_report", "json");

    if quality_plugins.is_empty() {
        println!("No quality analysis plugins configured. Available output plugins:");
        for (name, plugin_config) in config.get_enabled_output_plugins() {
            println!(
                "  {} - Types: {:?}, Formats: {:?}",
                name, plugin_config.output_types, plugin_config.formats
            );
        }
        return Ok(());
    }

    println!("Quality analysis functionality will be implemented using output plugins:");
    for plugin_name in &quality_plugins {
        println!("  - {plugin_name}");
    }

    // TODO: Implement quality analysis using output plugins
    println!("Quality analysis functionality will be implemented here");

    Ok(())
}

async fn handle_docs(
    matrix: Option<PathBuf>,
    format: crate::cli::args::DocFormat,
    output_dir: Option<PathBuf>,
    config: &Config,
) -> Result<()> {
    debug!("Generating documentation...");

    let matrix_path = matrix.unwrap_or_else(|| PathBuf::from(".csd_cache/matrix.json"));
    let output_directory = output_dir.unwrap_or_else(|| PathBuf::from(&config.output_dir));

    if !matrix_path.exists() {
        return Err(anyhow::anyhow!(
            "Matrix file not found: {}. Run 'csd scan' first.",
            matrix_path.display()
        ));
    }

    // Convert DocFormat to string
    let format_str = match format {
        crate::cli::args::DocFormat::Markdown => "markdown",
        crate::cli::args::DocFormat::Html => "html",
        crate::cli::args::DocFormat::Pdf => "pdf",
    };

    // Find documentation output plugins that support the requested format
    let doc_plugins = config.find_output_plugins_for_type("documentation", format_str);

    if doc_plugins.is_empty() {
        println!("No documentation plugins found for format '{format_str}'. Available plugins:");
        for (name, plugin_config) in config.get_enabled_output_plugins() {
            if plugin_config
                .output_types
                .contains(&"documentation".to_string())
            {
                println!("  {} - Formats: {:?}", name, plugin_config.formats);
            }
        }
        return Ok(());
    }

    info!("Generating documentation using plugins: {doc_plugins:?}");

    // Use the first available plugin for now
    let plugin_name = &doc_plugins[0];
    let plugin_config = config.get_output_plugin(plugin_name).unwrap();

    // Create the output directory
    tokio::fs::create_dir_all(&output_directory).await?;

    // Set up plugin communication
    use crate::plugins::communication::OutputPluginCommunicator;

    // Resolve plugin path
    let plugin_path = match &plugin_config.source {
        crate::utils::config::PluginSource::Builtin { name } => {
            PathBuf::from(format!("plugins/output/{name}.py"))
        }
        crate::utils::config::PluginSource::Local { path } => PathBuf::from(path),
        _ => {
            return Err(anyhow::anyhow!(
                "Plugin source type not yet supported: {:?}",
                plugin_config.source
            ));
        }
    };

    if !plugin_path.exists() {
        return Err(anyhow::anyhow!(
            "Output plugin file not found: {}",
            plugin_path.display()
        ));
    }

    // Create plugin input
    let plugin_input = OutputPluginInput {
        matrix_path: matrix_path.clone(),
        project_root: std::env::current_dir()?,
        output_dir: output_directory.clone(),
        cache_dir: ".csd_cache".to_string(),
        plugin_config: plugin_config
            .config
            .as_ref()
            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null)),
        format_options: serde_json::json!({
            "format": format_str,
            "output_type": "documentation"
        }),
    };

    // Create and configure communicator
    let mut communicator =
        OutputPluginCommunicator::new(plugin_path).with_cache_dir(PathBuf::from(".csd_cache"));

    if let Some(ref python_exe) = config.python_executable {
        communicator = communicator.with_python_executable(python_exe.clone());
    } else {
        communicator = communicator.with_python_auto_detect();
    }

    // Generate documentation
    match communicator.generate(plugin_input).await {
        Ok(result) => {
            info!("Documentation generated successfully!");
            println!(
                "📚 Documentation generated by {} v{}",
                result.plugin_name, result.plugin_version
            );
            println!("📁 Output directory: {}", output_directory.display());
            println!("📄 Generated {} files:", result.outputs.len());

            for output in &result.outputs {
                let size_kb = output.size_bytes as f64 / 1024.0;
                println!(
                    "   {} ({:.1} KB) - {}",
                    output.output_path.display(),
                    size_kb,
                    output.content_type
                );
            }

            println!("⏱️  Processing time: {}ms", result.processing_time_ms);
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Documentation generation failed: {}", e));
        }
    }

    Ok(())
}

async fn handle_plugins(detailed: bool, config: &Config) -> Result<()> {
    debug!("Listing available plugins...");

    let plugin_manager = PluginManager::new(config.clone());
    let plugins = plugin_manager.discover_plugins().await?;

    if detailed {
        println!("=== Input Plugins (Code Analyzers) ===");
        let input_plugins: Vec<_> = plugins
            .iter()
            .filter(|p| p.plugin_type == "input")
            .collect();

        if input_plugins.is_empty() {
            println!("No input plugins configured.");
        } else {
            for plugin in input_plugins {
                println!("Plugin: {}", plugin.name);
                println!("  Type: Input (Code Analyzer)");
                println!("  Path: {}", plugin.path.display());
                println!("  Extensions: {}", plugin.extensions.join(", "));
                println!("  Filenames: {}", plugin.filenames.join(", "));
                println!("  Source: {:?}", plugin.source);
                println!("  Enabled: {}", plugin.enabled);
                println!();
            }
        }

        println!("=== Output Plugins (Documentation Generators, etc.) ===");
        let output_plugins: Vec<_> = plugins
            .iter()
            .filter(|p| p.plugin_type == "output")
            .collect();

        if output_plugins.is_empty() {
            println!("No output plugins configured.");
        } else {
            for plugin in output_plugins {
                println!("Plugin: {}", plugin.name);
                println!("  Type: Output (Generator)");
                println!("  Path: {}", plugin.path.display());
                println!("  Output Types: {}", plugin.output_types.join(", "));
                println!("  Formats: {}", plugin.formats.join(", "));
                println!("  Source: {:?}", plugin.source);
                println!("  Enabled: {}", plugin.enabled);
                println!();
            }
        }
    } else {
        println!("Input Plugins:");
        for plugin in plugins.iter().filter(|p| p.plugin_type == "input") {
            let all_patterns: Vec<String> = plugin
                .extensions
                .iter()
                .chain(plugin.filenames.iter())
                .cloned()
                .collect();
            println!("  {} - {}", plugin.name, all_patterns.join(", "));
        }

        println!("\nOutput Plugins:");
        for plugin in plugins.iter().filter(|p| p.plugin_type == "output") {
            println!(
                "  {} - Types: {}, Formats: {}",
                plugin.name,
                plugin.output_types.join(","),
                plugin.formats.join(",")
            );
        }
    }

    // Show configuration summary
    let summary = config.get_plugin_summary();
    println!("\n📊 Plugin Summary:");
    println!(
        "  Input plugins: {} enabled / {} total",
        summary.enabled_input_plugins, summary.total_input_plugins
    );
    println!(
        "  Output plugins: {} enabled / {} total",
        summary.enabled_output_plugins, summary.total_output_plugins
    );

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

    println!("✅ Created configuration file: {}", config_path.display());

    let summary = default_config.get_plugin_summary();
    println!("📦 Default configuration includes:");
    println!(
        "  {} input plugins: {}",
        summary.total_input_plugins,
        summary.input_plugin_names.join(", ")
    );
    println!(
        "  {} output plugins: {}",
        summary.total_output_plugins,
        summary.output_plugin_names.join(", ")
    );

    Ok(())
}
