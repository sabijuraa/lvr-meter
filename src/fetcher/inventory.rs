use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

use crate::config::WalletAddress;
use crate::fetcher::pool_fetcher::fetch_pool_states;
use crate::fetcher::raydium::pool_state::PoolState;
use crate::fetcher::raydium::position_fetcher::fetch_positions;
use crate::fetcher::raydium::types::PersonalPositionState;
use crate::fetcher::rpc::RpcClientWrapper;

pub struct PositionInventory {
    pub positions:   Vec<PersonalPositionState>,
    pub pool_states: HashMap<Pubkey, PoolState>,
}

impl PositionInventory {
    pub fn fetch(wallet: &WalletAddress, client: &RpcClientWrapper) -> Result<Self> {
        tracing::info!("Fetching positions for wallet {}", wallet.as_str());
        let positions = fetch_positions(wallet, client)?;
        tracing::info!("Found {} positions", positions.len());

        let pool_ids: Vec<Pubkey> = positions
            .iter()
            .map(|p| p.pool_id)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        tracing::info!("Fetching {} unique pool states", pool_ids.len());
        let pool_states = fetch_pool_states(&pool_ids, client)?;

        Ok(Self { positions, pool_states })
    }

    pub fn position_count(&self) -> usize {
        self.positions.len()
    }

    pub fn pool_count(&self) -> usize {
        self.pool_states.len()
    }
}