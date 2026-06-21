mod cli;

use clap::Parser;
use cli::{Cli, OutputFormat};
use lvr_meter::config::Config;
use lvr_meter::engine::analysis::PositionAnalysis;
use lvr_meter::engine::optimizer::search::run_optimizer;
use lvr_meter::fetcher::pipeline::FetchPipeline;
use lvr_meter::output::historical_table::{AnalysisInput, print_historical_table};
use lvr_meter::output::json_output::print_json_output;
use lvr_meter::output::recommendation_table::print_recommendation_table;
use lvr_meter::output::summary::print_config_summary;
use lvr_meter::parser::batch::parse_pool_transactions;

fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // ── Phase 1: Config ───────────────────────────────────────────────────────
    let config = Config::from_env_and_args(
        &cli.wallet,
        &cli.from,
        &cli.to,
        &cli.protocol,
        cli.pool,
    )
    .unwrap_or_else(|e| fatal("Invalid configuration", e));

    tracing::info!("Configuration validated");

    if cli.dry_run {
        print_config_summary(&config);
        std::process::exit(0);
    }

    let date_range = config.date_range.clone();

    // ── Phase 2 + 3: Fetch pipeline ──────────────────────────────────────────
    let pipeline = FetchPipeline::new(config)
        .unwrap_or_else(|e| fatal("Failed to initialize fetch pipeline", e));

    let fetch_result = pipeline
        .run_for_dates(date_range.from_date(), date_range.to_date())
        .unwrap_or_else(|e| fatal("Fetch pipeline failed", e));

    tracing::info!(
        "Fetched {} transactions across {} pools",
        fetch_result.total_transactions(),
        fetch_result.pool_count()
    );

    if fetch_result.inventory.positions.is_empty() {
        eprintln!("No CLMM positions found for this wallet.");
        std::process::exit(0);
    }

    // ── Phase 4: Parse transactions ──────────────────────────────────────────
    let mut all_analyses: Vec<PositionAnalysis> = Vec::new();
    let mut analysis_inputs:  Vec<(String, PositionAnalysis)> = Vec::new();

    for position in &fetch_result.inventory.positions {
        let pool_id = position.pool_id;

        let pool_state = match fetch_result.inventory.pool_states.get(&pool_id) {
            Some(s) => s,
            None    => {
                tracing::warn!("No pool state for pool {}", pool_id);
                continue;
            }
        };

        let raw_txs = fetch_result
            .transactions
            .get(&pool_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        // parse_pool_transactions needs EncodedTransactionWithStatusMeta
        // raw_txs are EncodedTransaction — skipping parse for now, using empty slice
        // Full parsing wired in when Helius enhanced tx format is available
        let events = parse_pool_transactions(
            &[],
            &pool_id,
            pool_state,
            position,
        );

        tracing::info!(
            "Pool {}: {} swap events parsed",
            pool_id,
            events.len()
        );

        // ── Phase 5: Engine ───────────────────────────────────────────────────
        let analysis = PositionAnalysis::compute(
            &events,
            position,
            pool_state,
            pool_state,
        )
        .unwrap_or_else(|e| fatal("Engine computation failed", e));

        analysis_inputs.push((pool_id.to_string(), analysis.clone()));
        all_analyses.push(analysis);
    }

    if all_analyses.is_empty() {
        eprintln!("No analyses produced. Check that the wallet has active positions.");
        std::process::exit(0);
    }

    // ── Phase 6: Optimizer ────────────────────────────────────────────────────
    // Use regime from the first position's analysis as the representative regime
    let regime        = &all_analyses[0].regime;
    let optimizer_result = run_optimizer(regime, &[]);

    tracing::info!("{}", optimizer_result.recommendation_line());

    // ── Phase 7: Output ───────────────────────────────────────────────────────
    let current_price = fetch_result
        .inventory
        .pool_states
        .values()
        .next()
        .map(|ps| {
            lvr_meter::parser::price::sqrt_price_x64_to_price(
                ps.sqrt_price_x64,
                ps.mint_decimals_0,
                ps.mint_decimals_1,
            )
        })
        .unwrap_or(0.0);

    match cli.output {
        OutputFormat::Json => {
            print_json_output(&all_analyses, &optimizer_result)
                .unwrap_or_else(|e| fatal("JSON serialization failed", e));
        }

        OutputFormat::Table => {
            let inputs: Vec<AnalysisInput> = analysis_inputs
                .iter()
                .map(|(pool_id, analysis)| AnalysisInput {
                    pool_id:  pool_id.clone(),
                    period:   format!(
                        "{} → {}",
                        date_range.from_date(),
                        date_range.to_date()
                    ),
                    analysis,
                })
                .collect();

            print_historical_table(&inputs);
            print_recommendation_table(&optimizer_result, regime, current_price);
        }
    }

    tracing::info!("Analysis complete.");
}

/// Print a clean error message and exit with code 1.
/// Never shows a Rust backtrace to the user.
fn fatal<T>(context: &str, err: anyhow::Error) -> T {
    eprintln!("\nError: {}", context);
    eprintln!("  {}", err);

    // Walk the error chain
    let mut source = err.source();
    while let Some(cause) = source {
        eprintln!("  caused by: {}", cause);
        source = cause.source();
    }

    eprintln!("\nRun with RUST_LOG=debug for more detail.");
    std::process::exit(1);
}