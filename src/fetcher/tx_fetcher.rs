use anyhow::Result;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{
    EncodedTransaction, EncodedTransactionWithStatusMeta, UiTransactionEncoding,
};
use std::str::FromStr;

use crate::fetcher::cache::TxCache;
use crate::fetcher::rpc::RpcClientWrapper;

const SIGNATURES_PER_PAGE: usize = 1000;
const TX_BATCH_SIZE: usize       = 100;

pub fn fetch_transactions_for_pool(
    pool:       &Pubkey,
    slot_start: u64,
    slot_end:   u64,
    cache:      &TxCache,
    client:     &RpcClientWrapper,
) -> Result<Vec<EncodedTransaction>> {
    if cache.exists(pool, slot_start, slot_end) {
        tracing::info!("Cache hit for pool {} [{}-{}]", pool, slot_start, slot_end);
        return Ok(cache.get(pool, slot_start, slot_end).unwrap_or_default());
    }

    tracing::info!("Cache miss — fetching transactions for pool {}", pool);

    let mut all_txs: Vec<EncodedTransaction> = Vec::new();
    let mut before: Option<Signature>        = None;
    let mut page                             = 0usize;

    loop {
        page += 1;

        let config = GetConfirmedSignaturesForAddress2Config {
            before,
            until:      None,
            limit:      Some(SIGNATURES_PER_PAGE),
            commitment: Some(CommitmentConfig::confirmed()),
        };

        let sigs = client.call_with_retry("get_signatures_for_address", || {
            client
                .client
                .get_signatures_for_address_with_config(pool, config.clone())
        })?;

        if sigs.is_empty() {
            break;
        }

        // Filter to slot range
        let in_range: Vec<_> = sigs
            .iter()
            .filter(|s| {
                let slot = s.slot;
                slot >= slot_start && slot <= slot_end
            })
            .collect();

        let reached_start = sigs.last().map(|s| s.slot <= slot_start).unwrap_or(false);

        tracing::info!(
            "Page {} for pool {} — {} signatures in range, {} total so far",
            page,
            pool,
            in_range.len(),
            all_txs.len()
        );

        // Fetch full transactions in batches of 100
        let signatures: Vec<Signature> = in_range
            .iter()
            .filter_map(|s| Signature::from_str(&s.signature).ok())
            .collect();

        for chunk in signatures.chunks(TX_BATCH_SIZE) {
            let txs = fetch_transaction_batch(chunk, client)?;
            all_txs.extend(txs);
        }

        // Write this page to cache incrementally
        cache.set(pool, slot_start, slot_end, &all_txs)?;

        if reached_start || sigs.len() < SIGNATURES_PER_PAGE {
            break;
        }

        // Set cursor for next page
        before = sigs
            .last()
            .and_then(|s| Signature::from_str(&s.signature).ok());
    }

    tracing::info!(
        "Finished fetching pool {} — {} transactions total",
        pool,
        all_txs.len()
    );

    Ok(all_txs)
}

fn fetch_transaction_batch(
    signatures: &[Signature],
    client:     &RpcClientWrapper,
) -> Result<Vec<EncodedTransaction>> {
    let mut txs = Vec::new();

    for sig in signatures {
        let sig_copy = *sig;
        let result = client.call_with_retry("get_transaction", || {
            client.client.get_transaction(
                &sig_copy,
                UiTransactionEncoding::Json,
            )
        });

        match result {
            Ok(tx) => txs.push(tx.transaction.transaction),
            Err(e) => tracing::warn!("Failed to fetch tx {}: {}", sig, e),
        }
    }

    Ok(txs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_valid() {
        assert_eq!(SIGNATURES_PER_PAGE, 1000);
        assert_eq!(TX_BATCH_SIZE, 100);
        assert!(TX_BATCH_SIZE <= SIGNATURES_PER_PAGE);
    }

    #[test]
    fn chunks_signatures_correctly() {
        let sigs: Vec<Signature> = (0..250)
            .map(|_| Signature::default())
            .collect();

        let chunks: Vec<&[Signature]> = sigs.chunks(TX_BATCH_SIZE).collect();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 100);
        assert_eq!(chunks[1].len(), 100);
        assert_eq!(chunks[2].len(), 50);
    }
}