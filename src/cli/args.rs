use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "csd",
    about = "A comprehensive code analysis and documentation tool",
    version,
    author
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// Project root directory
    #[arg(short, long, global = true)]
    pub project: Option<PathBuf>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Scan a project and build the analysis matrix
    Scan {
        /// Path to the project directory
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,

        /// Output format for the results
        #[arg(short, long, default_value = "json")]
        output: OutputFormat,

        /// Output file path (defaults to stdout)
        #[arg(short = 'f', long)]
        output_file: Option<PathBuf>,

        /// Skip LLM analysis and only do structural analysis
        #[arg(long)]
        no_llm: bool,

        /// Include test files in analysis
        #[arg(long)]
        include_tests: bool,
    },

    /// Analyze code quality based on existing matrix
    Quality {
        /// Path to the matrix file
        #[arg(short, long)]
        matrix: Option<PathBuf>,

        /// Specific quality metrics to calculate
        #[arg(long)] // Removed short flag to avoid conflict with matrix
        metrics: Vec<QualityMetric>,
    },

    /// Generate documentation from analysis
    Docs {
        /// Path to the matrix file
        #[arg(short, long)]
        matrix: Option<PathBuf>,

        /// Documentation format
        #[arg(short, long, default_value = "markdown")]
        format: DocFormat,

        /// Output directory for documentation
        #[arg(short, long)]
        output_dir: Option<PathBuf>,
    },

    /// List available plugins
    Plugins {
        /// Show detailed plugin information
        #[arg(long)]
        detailed: bool,
    },

    /// Initialize a new configuration file
    Init {
        /// Force overwrite existing configuration
        #[arg(long)]
        force: bool,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Json,
    Yaml,
    Pretty,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum QualityMetric {
    Complexity,
    Coverage,
    Maintainability,
    Security,
    Performance,
    All,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum DocFormat {
    Markdown,
    Html,
    Pdf,
}
