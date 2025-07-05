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

    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    #[arg(short, long, global = true)]
    pub project: Option<PathBuf>,
}

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

    Quality {
        #[arg(short, long)]
        matrix: Option<PathBuf>,

        #[arg(long)]
        metrics: Vec<QualityMetric>,
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
