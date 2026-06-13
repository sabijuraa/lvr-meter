mod cli;
mod config;
mod output;

use clap::Parser;
use cli::Cli;
use config::Config;
use output::summary::print_config_summary;

fn main() {
    // Load .env file if it exists
    dotenvy::dotenv().ok();

    // Initialize tracing — reads RUST_LOG env var for log level
    tracing_subscriber::fmt::init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Validate into Config types
    let config = Config::from_env_and_args(
        &cli.wallet,
        &cli.from,
        &cli.to,
        &cli.protocol,
        cli.pool,
    );

    let config = match config {
        Ok(c)  => c,
        Err(e) => {
            tracing::error!("Invalid configuration: {e}");
            std::process::exit(1);
        }
    };

    tracing::info!("Configuration validated successfully");

    // Dry run — print summary and exit without network calls
    if cli.dry_run {
        print_config_summary(&config);
        std::process::exit(0);
    }

    
    tracing::info!("Starting analysis...");
    println!("Full analysis not yet implemented — use --dry-run");
}