use std::collections::HashSet;

use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedTransactionWithStatusMeta;

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
    let mut events: Vec<SwapEvent> = txs
        .iter()
        .enumerate()
        .filter_map(|(i, tx)| parse_swap_event(tx, pool, pool_state, i as u64, 0))
        .collect();

    let swap_count = events.len();

    let mut seen = HashSet::new();
    events.retain(|e| seen.insert((e.slot, e.sqrt_price_before, e.sqrt_price_after)));

    let dedup_count = events.len();

    let in_range = filter_swaps_to_position(
        &events,
        position.tick_lower_index,
        position.tick_upper_index,
    );

    let range_count = in_range.len();

    let mut sorted = in_range;
    sorted.sort_by_key(|e| e.slot);

    tracing::info!(
        "pool {}: {} transactions → {} swaps → {} deduped → {} in-range events",
        pool, txs.len(), swap_count, dedup_count, range_count,
    );

    sorted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::types::{SwapDirection, SwapEvent};

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
        let e2 = make_swap_event(100, 1.0, 1.001);
        assert!( seen.insert((e1.slot, e1.sqrt_price_before, e1.sqrt_price_after)));
        assert!(!seen.insert((e2.slot, e2.sqrt_price_before, e2.sqrt_price_after)));
    }

    #[test]
    fn empty_input_returns_empty() {
        let pool      = Pubkey::new_unique();
        let pool_st   = make_pool_state_stub();
        let position  = make_position_stub();
        let result    = parse_pool_transactions(&[], &pool, &pool_st, &position);
        assert_eq!(result.len(), 0);
    }

    fn make_pool_state_stub() -> PoolState {
        use crate::fetcher::raydium::pool_state::POOL_STATE_DISCRIMINATOR;
        let mut data = vec![0u8; 512];
        data[..8].copy_from_slice(&POOL_STATE_DISCRIMINATOR);
        data[8 + 245..8 + 261].copy_from_slice(&(1u128 << 64).to_le_bytes());
        PoolState::from_account_data(&data).unwrap()
    }

    fn make_position_stub() -> PersonalPositionState {
        PersonalPositionState {
            bump:                         [255],
            nft_mint:                     Pubkey::new_unique(),
            pool_id:                      Pubkey::new_unique(),
            tick_lower_index:             -100,
            tick_upper_index:              100,
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
}