use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{
    EncodedTransaction, EncodedTransactionWithStatusMeta, UiMessage,
};
use std::str::FromStr;

use crate::fetcher::raydium::pool_state::PoolState;
use crate::parser::account_state::{extract_pool_state_before_after, PoolStateSnapshot};
use crate::parser::price::sqrt_price_x64_to_price;
use crate::parser::raydium::discriminator::is_raydium_swap;
use crate::parser::types::{SwapDirection, SwapEvent};

pub fn parse_swap_event(
    tx:         &EncodedTransactionWithStatusMeta,
    pool:       &Pubkey,
    pool_state: &PoolState,
) -> Option<SwapEvent> {
    let sig = extract_signature(tx).unwrap_or_else(|| "unknown".to_string());

    // Confirm this transaction contains a Raydium swap instruction
    if !contains_raydium_swap(tx) {
        return None;
    }

    let slot      = tx.slot;
    let timestamp = tx.block_time.unwrap_or(0);

    let (pre, post) = match extract_pool_state_before_after(tx, pool) {
        Some(states) => states,
        None => {
            tracing::warn!(
                "Could not extract pool state for tx {} pool {}",
                sig, pool
            );
            return None;
        }
    };

    if pre.sqrt_price_x64 == post.sqrt_price_x64 {
        // Price did not move — not a swap that affected this pool
        return None;
    }

    let price_before = sqrt_price_x64_to_price(
        pre.sqrt_price_x64,
        pool_state.mint_decimals_0,
        pool_state.mint_decimals_1,
    );
    let price_after = sqrt_price_x64_to_price(
        post.sqrt_price_x64,
        pool_state.mint_decimals_0,
        pool_state.mint_decimals_1,
    );

    let direction = determine_direction(&pre, &post);

    Some(SwapEvent {
        slot,
        timestamp,
        pool:              *pool,
        price_before,
        price_after,
        sqrt_price_before: pre.sqrt_price_x64,
        sqrt_price_after:  post.sqrt_price_x64,
        active_liquidity:  pre.liquidity,
        fee_rate:          pool_state.tick_spacing,
        direction,
    })
}

/// Returns true if the transaction contains at least one Raydium CLMM swap instruction
fn contains_raydium_swap(tx: &EncodedTransactionWithStatusMeta) -> bool {
    let ui_tx = match &tx.transaction {
        EncodedTransaction::Json(t) => t,
        _ => return false,
    };

    let (account_keys, instructions) = match &ui_tx.message {
        UiMessage::Raw(msg) => {
            let keys: Vec<Pubkey> = msg
                .account_keys
                .iter()
                .filter_map(|k| Pubkey::from_str(k).ok())
                .collect();

            let instrs: Vec<(Pubkey, Vec<u8>)> = msg
                .instructions
                .iter()
                .filter_map(|ix| {
                    let prog_id = keys.get(ix.program_id_index as usize)?;
                    let data    = bs58::decode(&ix.data).into_vec().ok()?;
                    Some((*prog_id, data))
                })
                .collect();

            (keys, instrs)
        }
        _ => return false,
    };

    instructions
        .iter()
        .any(|(prog_id, data)| is_raydium_swap(prog_id, data))
}

/// Determine swap direction from price movement.
/// ZeroForOne = price goes down (selling token_0, buying token_1)
/// OneForZero = price goes up   (selling token_1, buying token_0)
fn determine_direction(pre: &PoolStateSnapshot, post: &PoolStateSnapshot) -> SwapDirection {
    if post.sqrt_price_x64 < pre.sqrt_price_x64 {
        SwapDirection::ZeroForOne
    } else {
        SwapDirection::OneForZero
    }
}

fn extract_signature(tx: &EncodedTransactionWithStatusMeta) -> Option<String> {
    if let EncodedTransaction::Json(ui_tx) = &tx.transaction {
        return ui_tx.signatures.first().cloned();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::account_state::PoolStateSnapshot;

    #[test]
    fn direction_zero_for_one_when_price_falls() {
        let pre  = make_snapshot(2_000_000);
        let post = make_snapshot(1_000_000);
        assert_eq!(determine_direction(&pre, &post), SwapDirection::ZeroForOne);
    }

    #[test]
    fn direction_one_for_zero_when_price_rises() {
        let pre  = make_snapshot(1_000_000);
        let post = make_snapshot(2_000_000);
        assert_eq!(determine_direction(&pre, &post), SwapDirection::OneForZero);
    }

    #[test]
    fn price_conversion_uses_pool_decimals() {
        let sqrt  = 1u128 << 64;
        let price = sqrt_price_x64_to_price(sqrt, 9, 6);
        // equal sqrt = ratio 1.0, adjustment 10^(9-6) = 1000
        assert!((price - 1000.0).abs() < 0.01);
    }

    fn make_snapshot(sqrt_price_x64: u128) -> PoolStateSnapshot {
        PoolStateSnapshot {
            sqrt_price_x64,
            liquidity:               1_000_000,
            tick_current:            0,
            fee_growth_global_0_x64: 0,
            fee_growth_global_1_x64: 0,
        }
    }
}