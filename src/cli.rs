use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name    = "lvr-meter",
    about   = "Measure LVR and fees for your Solana CLMM positions",
    version = "0.1.0"
)]
pub struct Cli {
    /// Solana wallet address to analyze
    #[arg(long)]
    pub wallet: String,

    /// Start date in YYYY-MM-DD format
    #[arg(long)]
    pub from: String,

    /// End date in YYYY-MM-DD format
    #[arg(long)]
    pub to: String,

    /// Protocol to analyze: raydium, orca, or both
    #[arg(long, default_value = "both")]
    pub protocol: String,

    /// Specific pool address to filter by (optional)
    #[arg(long)]
    pub pool: Option<String>,

    /// Print config summary and exit without hitting the network
    #[arg(long)]
    pub dry_run: bool,
}