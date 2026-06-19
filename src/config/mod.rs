pub mod date_range;
pub mod filters;
pub mod types;

pub use date_range::DateRange;
pub use filters::{PoolFilter, Protocol};
pub use types::WalletAddress;

use crate::fetcher::helius::HeliusClient;
use anyhow::{bail, Context, Result};
use std::env;
#[derive(Clone)]
pub struct Config {
    pub wallet:         WalletAddress,
    pub date_range:     DateRange,
    pub filter:         PoolFilter,
    pub rpc_url:        String,
    #[allow(dead_code)]
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

        if helius_api_key.is_empty() {
            bail!("HELIUS_API_KEY environment variable is empty");
        }

        // Build RPC URL through HeliusClient — single source of truth
        let helius  = HeliusClient::new(&helius_api_key);
        let rpc_url = helius.rpc_url();

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