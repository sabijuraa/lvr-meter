use clap::{Parser, ValueEnum};

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}

#[derive(Parser, Debug)]
#[command(
    name    = "lvr-meter",
    about   = "Measure LVR and fees for your Solana CLMM positions",
    version = "0.1.0"
)]
pub struct Cli {
    #[arg(long)]
    pub wallet: String,

    #[arg(long)]
    pub from: String,

    #[arg(long)]
    pub to: String,

    #[arg(long, default_value = "both")]
    pub protocol: String,

    #[arg(long)]
    pub pool: Option<String>,

    #[arg(long)]
    pub dry_run: bool,

    /// Output format: table (default) or json
    #[arg(long, value_enum, default_value = "table")]
    pub output: OutputFormat,
}