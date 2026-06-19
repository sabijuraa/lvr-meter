mod cli;

use clap::Parser;
use cli::Cli;
use lvr_meter::config::Config;
use lvr_meter::fetcher::inventory::PositionInventory;
use lvr_meter::fetcher::rpc::RpcClientWrapper;
use lvr_meter::output::position_table::{print_position_inventory, PositionRow};
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
        Ok(c) => c,
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

    let client    = RpcClientWrapper::new(&config.rpc_url);
    let inventory = match PositionInventory::fetch(&config.wallet, &client) {
        Ok(inv) => inv,
        Err(e)  => {
            tracing::error!("Failed to fetch position inventory: {e}");
            std::process::exit(1);
        }
    };

    tracing::info!(
        "Loaded {} positions across {} pools",
        inventory.position_count(),
        inventory.pool_count()
    );

    let rows: Vec<PositionRow> = inventory
        .positions
        .iter()
        .map(|p| PositionRow {
            pool_id:      truncate_pubkey(&p.pool_id.to_string()),
            tick_lower:   p.tick_lower_index,
            tick_upper:   p.tick_upper_index,
            liquidity:    p.liquidity,
            fee_rate_bps: 0,
        })
        .collect();

    print_position_inventory(&rows);
}

fn truncate_pubkey(s: &str) -> String {
    if s.len() <= 12 {
        return s.to_string();
    }
    format!("{}..{}", &s[..6], &s[s.len() - 4..])
}