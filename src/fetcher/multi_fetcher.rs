use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedTransaction;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::task::JoinSet;

use crate::fetcher::cache::TxCache;
use crate::fetcher::inventory::PositionInventory;
use crate::fetcher::rpc::RpcClientWrapper;
use crate::fetcher::tx_fetcher::fetch_transactions_for_pool;

const MAX_CONCURRENT_POOLS: usize = 3;

pub async fn fetch_all_pools(
    inventory:  &PositionInventory,
    slot_start: u64,
    slot_end:   u64,
    cache:      Arc<TxCache>,
    client:     Arc<RpcClientWrapper>,
) -> Result<HashMap<Pubkey, Vec<EncodedTransaction>>> {
    let pool_ids: Vec<Pubkey> = inventory
        .positions
        .iter()
        .map(|p| p.pool_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    tracing::info!(
        "Fetching transactions for {} pools [{}-{}]",
        pool_ids.len(), slot_start, slot_end
    );

    let mut results: HashMap<Pubkey, Vec<EncodedTransaction>> = HashMap::new();

    for chunk in pool_ids.chunks(MAX_CONCURRENT_POOLS) {
        let mut set: JoinSet<(Pubkey, Result<Vec<EncodedTransaction>>)> = JoinSet::new();

        for &pool in chunk {
            let cache_ref  = Arc::clone(&cache);
            let client_ref = Arc::clone(&client);

            set.spawn(async move {
                let txs = tokio::task::spawn_blocking(
                    move || -> Result<Vec<EncodedTransaction>> {
                        fetch_transactions_for_pool(
                            &pool,
                            slot_start,
                            slot_end,
                            &cache_ref,
                            &client_ref,
                        )
                    },
                )
                .await
                .unwrap_or_else(|e| Err(anyhow::anyhow!("Task panicked: {}", e)));

                (pool, txs)
            });
        }

        while let Some(join_result) = set.join_next().await {
            match join_result {
                Ok((pool, Ok(txs))) => {
                    tracing::info!("Pool {} — {} transactions fetched", pool, txs.len());
                    results.insert(pool, txs);
                }
                Ok((pool, Err(e))) => {
                    tracing::error!("Failed to fetch pool {}: {}", pool, e);
                    results.insert(pool, vec![]);
                }
                Err(e) => {
                    tracing::error!("Task join error: {}", e);
                }
            }
        }

        tracing::info!("Progress: {}/{} pools fetched", results.len(), pool_ids.len());
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_concurrent_pools_is_reasonable() {
        assert!(MAX_CONCURRENT_POOLS >= 1);
        assert!(MAX_CONCURRENT_POOLS <= 10);
    }

    #[test]
    fn deduplicates_pool_ids() {
        let ids = vec![Pubkey::new_unique(), Pubkey::new_unique()];
        let mut seen = HashSet::new();
        for id in &ids { seen.insert(*id); }
        for id in &ids { seen.insert(*id); }
        assert_eq!(seen.len(), 2);
    }

    #[test]
    fn chunks_pools_correctly() {
        let pools: Vec<Pubkey> = (0..7).map(|_| Pubkey::new_unique()).collect();
        let chunks: Vec<&[Pubkey]> = pools.chunks(MAX_CONCURRENT_POOLS).collect();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 3);
        assert_eq!(chunks[1].len(), 3);
        assert_eq!(chunks[2].len(), 1);
    }
}