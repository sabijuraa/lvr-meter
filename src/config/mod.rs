pub mod date_range;
pub mod filters;
pub mod types;

pub use date_range::DateRange;
pub use filters::{PoolFilter, Protocol};
pub use types::WalletAddress;

use anyhow::{Context, Result};
use std::env;

pub struct Config {
    pub wallet:         WalletAddress,
    pub date_range:     DateRange,
    pub filter:         PoolFilter,
    pub rpc_url:        String,
    pub helius_api_key: String,
}

impl Config {
    pub fn from_env_and_args(
        wallet:        &str,
        from:          &str,
        to:            &str,
        protocol:      &str,
        specific_pool: Option<String>,
    ) -> Result<Self> {
        let helius_api_key = env::var("HELIUS_API_KEY")
            .context("HELIUS_API_KEY environment variable not set")?;

        let rpc_url = format!(
            "https://mainnet.helius-rpc.com/?api-key={}",
            helius_api_key
        );

        let wallet     = WalletAddress::parse(wallet)?;
        let date_range = DateRange::parse(from, to)?;
        let protocol   = protocol.parse::<Protocol>()?;
        let filter     = PoolFilter::new(protocol, specific_pool)?;

        Ok(Self {
            wallet,
            date_range,
            filter,
            rpc_url,
            helius_api_key,
        })
    }
}