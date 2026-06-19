mod cli;

use clap::Parser;
use cli::Cli;
use lvr_meter::config::Config;
use lvr_meter::fetcher::pipeline::FetchPipeline;
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

    let date_range = config.date_range.clone();

    let pipeline = match FetchPipeline::new(config) {
        Ok(p)  => p,
        Err(e) => {
            tracing::error!("Failed to initialize fetch pipeline: {e}");
            std::process::exit(1);
        }
    };

    let result = match pipeline.run_for_dates(
        date_range.from_date(),
        date_range.to_date(),
    ) {
        Ok(r)  => r,
        Err(e) => {
            tracing::error!("Fetch pipeline failed: {e}");
            std::process::exit(1);
        }
    };

    tracing::info!(
        "Fetched {} transactions across {} pools",
        result.total_transactions(),
        result.pool_count()
    );

    let rows: Vec<PositionRow> = result
        .inventory
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

    println!(
        "\nFetched {} swap transactions across {} pools, cached to .lvr-cache/",
        result.total_transactions(),
        result.pool_count()
    );
}

fn truncate_pubkey(s: &str) -> String {
    if s.len() <= 12 {
        return s.to_string();
    }
    format!("{}..{}", &s[..6], &s[s.len() - 4..])
}