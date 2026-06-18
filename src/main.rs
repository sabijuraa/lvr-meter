mod cli;

use clap::Parser;
use cli::Cli;
use lvr_meter::config::Config;
use lvr_meter::output::summary::print_config_summary;

fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

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

    if cli.dry_run {
        print_config_summary(&config);
        std::process::exit(0);
    }

    tracing::info!("Starting analysis...");
    println!("Full analysis not yet implemented — use --dry-run");
}