mod cli;
mod config;

use clap::Parser;
use cli::Cli;
use config::Config;

fn main() {
    // Load .env file if it exists
    dotenvy::dotenv().ok();

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

    match config {
        Ok(c) => {
            println!("=== lvr-meter config ===");
            println!("Wallet:   {}", c.wallet.as_str());
            println!("From:     {}", c.date_range.from_date());
            println!("To:       {}", c.date_range.to_date());
            println!("Days:     {}", c.date_range.num_days());
            println!("Protocol: {:?}", c.filter.protocol);
            println!("RPC URL:  {}", c.rpc_url);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}