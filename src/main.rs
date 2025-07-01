use clap::Parser;
use log::{error, info};

use csd::cli::args::Args;
use csd::cli::commands;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = Args::parse();
    info!("Starting code-scan-docs v{}", env!("CARGO_PKG_VERSION"));

    match commands::handle_command(args).await {
        Ok(_) => {
            info!("Command completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Command failed: {e}");
            std::process::exit(1);
        }
    }
}
