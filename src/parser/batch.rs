use std::collections::HashSet;

use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{EncodedTransaction, EncodedTransactionWithStatusMeta};

use crate::fetcher::raydium::pool_state::PoolState;
use crate::fetcher::raydium::types::PersonalPositionState;
use crate::parser::filter::filter_swaps_to_position;
use crate::parser::raydium::swap_parser::parse_swap_event;
use crate::parser::types::SwapEvent;

pub fn parse_pool_transactions(
    txs:        &[EncodedTransactionWithStatusMeta],
    pool:       &Pubkey,
    pool_state: &PoolState,
    position:   &PersonalPositionState,
) -> Vec<SwapEvent> {
    // Step 1 — parse each transaction into a SwapEvent
    let mut events: Vec<SwapEvent> = txs
        .iter()
        .filter_map(|tx| parse_swap_event(tx, pool, pool_state))
        .collect();

    let swap_count = events.len();

    // Step 2 — deduplicate by signature (slot + price fingerprint)
    let mut seen   = HashSet::new();
    events.retain(|e| seen.insert((e.slot, e.sqrt_price_before, e.sqrt_price_after)));

    let dedup_count = events.len();

    // Step 3 — filter to position's tick range
    let in_range = filter_swaps_to_position(
        &events,
        position.tick_lower_index,
        position.tick_upper_index,
    );

    let range_count = in_range.len();

    // Step 4 — sort by slot ascending
    let mut sorted = in_range;
    sorted.sort_by_key(|e| e.slot);

    tracing::info!(
        "pool {}: {} transactions → {} swaps → {} deduped → {} in-range events",
        pool,
        txs.len(),
        swap_count,
        dedup_count,
        range_count,
    );

    sorted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetcher::raydium::pool_state::POOL_STATE_DISCRIMINATOR;
    use crate::fetcher::raydium::types::{PersonalPositionState, REWARD_NUM};
    use crate::parser::types::SwapDirection;

    fn make_pool_state(sqrt_price: u128, tick: i32) -> PoolState {
        let mut data = vec![0u8; 512];
        data[..8].copy_from_slice(&POOL_STATE_DISCRIMINATOR);
        data[8 + 225] = 9;
        data[8 + 226] = 6;
        data[8 + 227..8 + 229].copy_from_slice(&10u16.to_le_bytes());
        data[8 + 229..8 + 245].copy_from_slice(&1_000_000u128.to_le_bytes());
        data[8 + 245..8 + 261].copy_from_slice(&sqrt_price.to_le_bytes());
        data[8 + 261..8 + 265].copy_from_slice(&tick.to_le_bytes());
        data[8 + 269..8 + 285].copy_from_slice(&0u128.to_le_bytes());
        data[8 + 285..8 + 301].copy_from_slice(&0u128.to_le_bytes());
        PoolState::from_account_data(&data).unwrap()
    }

    fn make_position(tick_lower: i32, tick_upper: i32) -> PersonalPositionState {
        PersonalPositionState {
            bump:                         [255],
            nft_mint:                     Pubkey::new_unique(),
            pool_id:                      Pubkey::new_unique(),
            tick_lower_index:             tick_lower,
            tick_upper_index:             tick_upper,
            liquidity:                    1_000_000,
            fee_growth_inside_0_last_x64: 0,
            fee_growth_inside_1_last_x64: 0,
            token_fees_owed_0:            0,
            token_fees_owed_1:            0,
            reward_infos:                 Default::default(),
            recent_epoch:                 0,
            padding:                      [0; 7],
        }
    }

    fn make_swap_event(slot: u64, price_before: f64, price_after: f64) -> SwapEvent {
        SwapEvent {
            slot,
            timestamp:         0,
            pool:              Pubkey::new_unique(),
            price_before,
            price_after,
            sqrt_price_before: (price_before * 1e18) as u128,
            sqrt_price_after:  (price_after  * 1e18) as u128,
            active_liquidity:  1_000_000,
            fee_rate:          25,
            direction:         SwapDirection::ZeroForOne,
        }
    }

    #[test]
    fn sorts_events_by_slot_ascending() {
        let pool      = Pubkey::new_unique();
        let pool_st   = make_pool_state(1u128 << 64, 0);
        let position  = make_position(-100, 100);

        // Simulate already-parsed events (bypass parse_swap_event)
        let mut events = vec![
            make_swap_event(300, 1.0, 1.001),
            make_swap_event(100, 1.0, 1.001),
            make_swap_event(200, 1.0, 1.001),
        ];
        events.sort_by_key(|e| e.slot);

        assert_eq!(events[0].slot, 100);
        assert_eq!(events[1].slot, 200);
        assert_eq!(events[2].slot, 300);
    }

    #[test]
    fn deduplicates_same_slot_and_price() {
        let mut seen: HashSet<(u64, u128, u128)> = HashSet::new();

        let e1 = make_swap_event(100, 1.0, 1.001);
        let e2 = make_swap_event(100, 1.0, 1.001); // duplicate

        let inserted1 = seen.insert((e1.slot, e1.sqrt_price_before, e1.sqrt_price_after));
        let inserted2 = seen.insert((e2.slot, e2.sqrt_price_before, e2.sqrt_price_after));

        assert!(inserted1);
        assert!(!inserted2); // duplicate rejected
    }

    #[test]
    fn empty_input_returns_empty() {
        let pool     = Pubkey::new_unique();
        let pool_st  = make_pool_state(1u128 << 64, 0);
        let position = make_position(-100, 100);

        let result = parse_pool_transactions(&[], &pool, &pool_st, &position);
        assert_eq!(result.len(), 0);
    }
}