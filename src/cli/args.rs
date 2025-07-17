//! Command-line argument parsing and configuration.
//!
//! This module defines the command-line interface structure using clap, including
//! all available commands, options, and their associated data types.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Main command-line arguments structure.
///
/// Represents the top-level arguments and global options that can be passed
/// to the application, along with the specific subcommand to execute.
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

    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    #[arg(short, long, global = true)]
    pub project: Option<PathBuf>,
}

/// Available subcommands for the application.
///
/// Each variant represents a different operation mode of the code analysis tool,
/// with its own specific arguments and options.
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    Init {
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,

        #[arg(short, long, default_value = "json")]
        output: OutputFormat,

        #[arg(short = 'f', long)]
        output_file: Option<PathBuf>,

        #[arg(long)]
        no_llm: bool,

        #[arg(long)]
        include_tests: bool,
    },

    Docs {
        #[arg(short, long)]
        matrix: Option<PathBuf>,

        #[arg(short, long, default_value = "markdown")]
        format: DocFormat,

        #[arg(short, long)]
        output_dir: Option<PathBuf>,
    },

    Plugins {
        #[arg(long)]
        detailed: bool,
    },

    Config {
        #[arg(long)]
        force: bool,
    },

    /// Analyze a diff and categorize changes into conventional commit types.
    ContextualChange {
        /// Path to the diff file to analyze.
        #[arg(value_name = "DIFF_PATH")]
        diff_path: PathBuf,
    },
}

/// Output format options for generated content.
///
/// Specifies the format in which analysis results or documentation
/// should be generated and saved.
#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Json,
    Yaml,
    Pretty,
}

/// Documentation format options.
///
/// Specifies the output format for generated documentation files.
#[derive(ValueEnum, Clone, Debug)]
pub enum DocFormat {
    Markdown,
    Html,
    Pdf,
}
