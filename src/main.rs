//! Code Scan Docs application entry point.
//!
//! This binary provides command-line interface functionality for scanning and
//! documenting code repositories.

use clap::Parser;
use log::{error, info};

use csd::cli::args::Args;
use csd::cli::commands;

/// Application entry point.
///
/// Initializes logging, parses command-line arguments, and executes the appropriate
/// command based on user input.
///
/// # Returns
///
/// Returns `Ok(())` on successful command execution, or an error if the command fails.
/// On error, the process exits with code 1.
///
/// # Examples
///
/// The application is typically run from the command line:
/// ```bash
/// cargo run -- [COMMAND] [OPTIONS]
/// ```
#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
