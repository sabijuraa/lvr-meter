use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{
    EncodedTransactionWithStatusMeta, UiTransaction, UiTransactionStatusMeta,
};

use crate::fetcher::raydium::pool_state::PoolState;

/// Minimal snapshot of PoolState fields needed for LVR computation
#[derive(Debug, Clone)]
pub struct PoolStateSnapshot {
    pub sqrt_price_x64:          u128,
    pub liquidity:               u128,
    pub tick_current:            i32,
    pub fee_growth_global_0_x64: u128,
    pub fee_growth_global_1_x64: u128,
}

impl PoolStateSnapshot {
    pub fn from_pool_state(state: &PoolState) -> Self {
        Self {
            sqrt_price_x64:          state.sqrt_price_x64,
            liquidity:               state.liquidity,
            tick_current:            state.tick_current,
            fee_growth_global_0_x64: state.fee_growth_global_0_x64,
            fee_growth_global_1_x64: state.fee_growth_global_1_x64,
        }
    }
}

/// Extract pre and post PoolState snapshots from a transaction.
///
/// Helius enhanced transactions include `pre_loaded_addresses` and account
/// data in `meta`. We find the pool account index, then deserialize its
/// pre/post data from the transaction's account data arrays.
///
/// Returns None if the pool address is not found in the transaction or
/// if account data is unavailable.
pub fn extract_pool_state_before_after(
    tx:           &EncodedTransactionWithStatusMeta,
    pool_address: &Pubkey,
) -> Option<(PoolStateSnapshot, PoolStateSnapshot)> {
    let meta = tx.meta.as_ref()?;

    let account_keys = extract_account_keys(tx)?;

    let pool_index = account_keys
        .iter()
        .position(|k| k == pool_address)?;

    let pre_data  = extract_account_data_at_index(meta, pool_index, true)?;
    let post_data = extract_account_data_at_index(meta, pool_index, false)?;

    let pre_state  = PoolState::from_account_data(&pre_data).ok()?;
    let post_state = PoolState::from_account_data(&post_data).ok()?;

    Some((
        PoolStateSnapshot::from_pool_state(&pre_state),
        PoolStateSnapshot::from_pool_state(&post_state),
    ))
}

/// Extract all account keys from a transaction (static + dynamic)
fn extract_account_keys(tx: &EncodedTransactionWithStatusMeta) -> Option<Vec<Pubkey>> {
    use solana_transaction_status::UiTransactionEncoding;

    if let solana_transaction_status::EncodedTransaction::Json(ui_tx) = &tx.transaction {
        if let solana_transaction_status::UiMessage::Raw(raw_msg) = &ui_tx.message {
            return Some(
                raw_msg
                    .account_keys
                    .iter()
                    .filter_map(|k| k.parse().ok())
                    .collect(),
            );
        }

        if let solana_transaction_status::UiMessage::Parsed(parsed_msg) = &ui_tx.message {
            return Some(
                parsed_msg
                    .account_keys
                    .iter()
                    .filter_map(|k| k.pubkey.parse().ok())
                    .collect(),
            );
        }
    }

    None
}

/// Extract raw account data at a given index from pre or post account data
/// stored in the transaction meta's account_data field.
///
/// Helius enhanced transactions store pre/post account data in
/// `meta.pre_accounts` and `meta.post_accounts` as base64-encoded bytes.
fn extract_account_data_at_index(
    meta:    &UiTransactionStatusMeta,
    index:   usize,
    is_pre:  bool,
) -> Option<Vec<u8>> {
    use solana_account_decoder::UiAccountData;

    let accounts = if is_pre {
        meta.pre_accounts.as_ref()?
    } else {
        meta.post_accounts.as_ref()?
    };

    let account = accounts.get(index)?;

    match &account.data {
        UiAccountData::Binary(encoded, encoding) => {
            use solana_account_decoder::UiAccountEncoding;
            match encoding {
                UiAccountEncoding::Base64 => {
                    use base64::{engine::general_purpose::STANDARD, Engine};
                    STANDARD.decode(encoded).ok()
                }
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetcher::raydium::pool_state::POOL_STATE_DISCRIMINATOR;

    fn build_pool_data(sqrt_price: u128, tick: i32, liquidity: u128) -> Vec<u8> {
        let mut data = vec![0u8; 512];
        data[..8].copy_from_slice(&POOL_STATE_DISCRIMINATOR);
        data[8 + 225] = 9;
        data[8 + 226] = 6;
        data[8 + 227..8 + 229].copy_from_slice(&10u16.to_le_bytes());
        data[8 + 229..8 + 245].copy_from_slice(&liquidity.to_le_bytes());
        data[8 + 245..8 + 261].copy_from_slice(&sqrt_price.to_le_bytes());
        data[8 + 261..8 + 265].copy_from_slice(&tick.to_le_bytes());
        data[8 + 269..8 + 285].copy_from_slice(&0u128.to_le_bytes());
        data[8 + 285..8 + 301].copy_from_slice(&0u128.to_le_bytes());
        data
    }

    #[test]
    fn snapshot_from_pool_state() {
        let data  = build_pool_data(12345678, -100, 999_000);
        let state = PoolState::from_account_data(&data).unwrap();
        let snap  = PoolStateSnapshot::from_pool_state(&state);

        assert_eq!(snap.sqrt_price_x64, 12345678);
        assert_eq!(snap.tick_current, -100);
        assert_eq!(snap.liquidity, 999_000);
    }

    #[test]
    fn snapshot_fields_match_pool_state() {
        let sqrt  = 18_446_744_073_709_551_616u128;
        let data  = build_pool_data(sqrt, 42, 5_000_000);
        let state = PoolState::from_account_data(&data).unwrap();
        let snap  = PoolStateSnapshot::from_pool_state(&state);

        assert_eq!(snap.sqrt_price_x64, state.sqrt_price_x64);
        assert_eq!(snap.tick_current,   state.tick_current);
        assert_eq!(snap.liquidity,      state.liquidity);
        assert_eq!(snap.fee_growth_global_0_x64, state.fee_growth_global_0_x64);
        assert_eq!(snap.fee_growth_global_1_x64, state.fee_growth_global_1_x64);
    }

    #[test]
    fn two_snapshots_can_differ() {
        let pre_data  = build_pool_data(1_000_000, -10, 1_000_000);
        let post_data = build_pool_data(2_000_000, -9,  1_000_000);

        let pre  = PoolStateSnapshot::from_pool_state(
            &PoolState::from_account_data(&pre_data).unwrap()
        );
        let post = PoolStateSnapshot::from_pool_state(
            &PoolState::from_account_data(&post_data).unwrap()
        );

        assert_ne!(pre.sqrt_price_x64, post.sqrt_price_x64);
        assert_ne!(pre.tick_current,   post.tick_current);
    }
}