use clap::Parser;
use env_logger;
use log::{info, error};

mod cli;
mod core;
mod plugins;
mod llm;
mod output;
mod utils;

use cli::args::Args;
use cli::commands;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    // Parse command line arguments
    let args = Args::parse();

    info!("Starting code-scan-docs v{}", env!("CARGO_PKG_VERSION"));

    // Handle the command
    match commands::handle_command(args).await {
        Ok(_) => {
            info!("Command completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Command failed: {}", e);
            std::process::exit(1);
        }
    }
}
