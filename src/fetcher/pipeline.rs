use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedTransaction;
use std::collections::HashMap;
use std::sync::Arc;

use crate::config::{Config, WalletAddress};
use crate::fetcher::cache::TxCache;
use crate::fetcher::inventory::PositionInventory;
use crate::fetcher::multi_fetcher::fetch_all_pools;
use crate::fetcher::rpc::RpcClientWrapper;
use crate::fetcher::slot_time::{date_to_unix_ts, estimate_slot_for_timestamp};

pub struct FetchResult {
    pub inventory:    PositionInventory,
    pub transactions: HashMap<Pubkey, Vec<EncodedTransaction>>,
}

impl FetchResult {
    pub fn total_transactions(&self) -> usize {
        self.transactions.values().map(|v| v.len()).sum()
    }

    pub fn pool_count(&self) -> usize {
        self.transactions.len()
    }
}

pub struct FetchPipeline {
    client:   Arc<RpcClientWrapper>,
    cache:    Arc<TxCache>,
    wallet:   WalletAddress,
    #[allow(dead_code)]
    config:   Arc<Config>,
}

impl FetchPipeline {
    pub fn new(config: Config) -> Result<Self> {
        Self::new_with_options(config, false)
    }

    pub fn new_with_options(config: Config, no_cache: bool) -> Result<Self> {
        let client = Arc::new(RpcClientWrapper::new(&config.rpc_url));
        let cache  = {
            let c = TxCache::default_dir()?;
            Arc::new(if no_cache { c.with_no_cache() } else { c })
        };
        let wallet = WalletAddress::parse(config.wallet.as_str())?;
        let config = Arc::new(config);

        Ok(Self { client, cache, wallet, config })
    }

    pub fn run(&self, slot_start: u64, slot_end: u64) -> Result<FetchResult> {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(self.run_async(slot_start, slot_end))
    }

    async fn run_async(&self, slot_start: u64, slot_end: u64) -> Result<FetchResult> {
        tracing::info!(
            "FetchPipeline: wallet={} slots=[{}-{}]",
            self.wallet.as_str(), slot_start, slot_end
        );

        let inventory = PositionInventory::fetch(&self.wallet, &self.client)?;

        tracing::info!(
            "Inventory: {} positions across {} pools",
            inventory.position_count(),
            inventory.pool_count()
        );

        let transactions = fetch_all_pools(
            &inventory,
            slot_start,
            slot_end,
            Arc::clone(&self.cache),
            Arc::clone(&self.client),
        )
        .await?;

        tracing::info!(
            "Fetch complete: {} transactions across {} pools",
            transactions.values().map(|v| v.len()).sum::<usize>(),
            transactions.len()
        );

        Ok(FetchResult { inventory, transactions })
    }

    pub fn run_for_dates(
        &self,
        from: chrono::NaiveDate,
        to:   chrono::NaiveDate,
    ) -> Result<FetchResult> {
        let slot_start = estimate_slot_for_timestamp(date_to_unix_ts(from), &self.client)?;
        let slot_end   = estimate_slot_for_timestamp(date_to_unix_ts(to),   &self.client)?;

        tracing::info!(
            "Date range [{} → {}] mapped to slots [{} → {}]",
            from, to, slot_start, slot_end
        );

        self.run(slot_start, slot_end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    #[test]
    fn fetch_result_counts() {
        let mut txs: HashMap<Pubkey, Vec<EncodedTransaction>> = HashMap::new();
        txs.insert(Pubkey::new_unique(), vec![]);
        txs.insert(Pubkey::new_unique(), vec![]);

        let result = FetchResult {
            inventory:    PositionInventory {
                positions:   vec![],
                pool_states: HashMap::new(),
            },
            transactions: txs,
        };

        assert_eq!(result.total_transactions(), 0);
        assert_eq!(result.pool_count(), 2);
    }
}