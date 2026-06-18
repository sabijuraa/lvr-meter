mod cli;

use clap::Parser;
use cli::Cli;
use lvr_meter::config::Config;
use lvr_meter::output::summary::print_config_summary;
use lvr_meter::output::position_table::{PositionRow, print_position_inventory};
use lvr_meter::fetcher::raydium::position_fetcher::fetch_positions;
use lvr_meter::fetcher::rpc::RpcClientWrapper;

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

    // Phase 2 — fetch positions
    let client = RpcClientWrapper::new(&config.rpc_url);

    let positions = match fetch_positions(&config.wallet, &client) {
        Ok(p)  => p,
        Err(e) => {
            tracing::error!("Failed to fetch positions: {e}");
            std::process::exit(1);
        }
    };

    let rows: Vec<PositionRow> = positions
        .iter()
        .map(|p| PositionRow {
            pool_id:      truncate_pubkey(&p.pool_id.to_string()),
            tick_lower:   p.tick_lower_index,
            tick_upper:   p.tick_upper_index,
            liquidity:    p.liquidity,
            fee_rate_bps: 0, // populated in Phase 2 completion — pool state fetch
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