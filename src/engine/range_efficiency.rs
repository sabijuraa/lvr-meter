use crate::parser::filter::tick_to_price;
use crate::parser::types::SwapEvent;

/// Compute the fraction of time the price spent within the position's tick range.
///
/// Algorithm:
///   1. Build a price timeline from all_events_in_window (all swaps in the pool)
///   2. For each consecutive pair of events, the price between them equals
///      price_after of the earlier event
///   3. Sum the slot gaps where that price was inside [price_lower, price_upper]
///   4. Divide by total slot span of the window
///
/// Returns a value in [0.0, 1.0].
/// Returns 0.0 if no events or zero slot span.
pub fn compute_range_efficiency(
    all_events_in_window: &[SwapEvent],
    tick_lower:           i32,
    tick_upper:           i32,
) -> f64 {
    if all_events_in_window.len() < 2 {
        return 0.0;
    }

    let price_lower = tick_to_price(tick_lower);
    let price_upper = tick_to_price(tick_upper);

    let first_slot = all_events_in_window.first().unwrap().slot;
    let last_slot  = all_events_in_window.last().unwrap().slot;
    let total_slots = last_slot.saturating_sub(first_slot);

    if total_slots == 0 {
        return 0.0;
    }

    let mut in_range_slots = 0u64;

    // Walk consecutive event pairs
    for window in all_events_in_window.windows(2) {
        let current = &window[0];
        let next    = &window[1];

        let slot_gap = next.slot.saturating_sub(current.slot);

        // The price between current and next equals current.price_after
        // (the pool price after the current swap settled)
        if is_in_range(current.price_after, price_lower, price_upper) {
            in_range_slots += slot_gap;
        }
    }

    in_range_slots as f64 / total_slots as f64
}

fn is_in_range(price: f64, lower: f64, upper: f64) -> bool {
    price >= lower && price <= upper
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::types::{SwapDirection, SwapEvent};
    use solana_sdk::pubkey::Pubkey;

    fn make_event(slot: u64, price_before: f64, price_after: f64) -> SwapEvent {
        SwapEvent {
            slot,
            timestamp:         0,
            pool:              Pubkey::new_unique(),
            price_before,
            price_after,
            sqrt_price_before: 0,
            sqrt_price_after:  0,
            active_liquidity:  1_000_000,
            fee_rate:          25,
            direction:         if price_after < price_before {
                SwapDirection::ZeroForOne
            } else {
                SwapDirection::OneForZero
            },
        }
    }

    // Tick range [-100, 100] — prices approximately [0.990, 1.010]
    const TICK_LOWER: i32 = -100;
    const TICK_UPPER: i32 =  100;

    fn range_mid() -> f64 {
        (tick_to_price(TICK_LOWER) + tick_to_price(TICK_UPPER)) / 2.0
    }

    fn below_range() -> f64 {
        tick_to_price(TICK_LOWER) * 0.5
    }

    fn above_range() -> f64 {
        tick_to_price(TICK_UPPER) * 2.0
    }

    #[test]
    fn empty_events_returns_zero() {
        let eff = compute_range_efficiency(&[], TICK_LOWER, TICK_UPPER);
        assert_eq!(eff, 0.0);
    }

    #[test]
    fn single_event_returns_zero() {
        let events = vec![make_event(100, range_mid(), range_mid())];
        let eff    = compute_range_efficiency(&events, TICK_LOWER, TICK_UPPER);
        assert_eq!(eff, 0.0);
    }

    #[test]
    fn all_time_in_range_returns_one() {
        // All events stay inside the range
        let mid    = range_mid();
        let events = vec![
            make_event(0,   mid, mid),
            make_event(100, mid, mid),
            make_event(200, mid, mid),
        ];
        let eff = compute_range_efficiency(&events, TICK_LOWER, TICK_UPPER);
        assert!((eff - 1.0).abs() < 1e-9, "Expected 1.0, got {}", eff);
    }

    #[test]
    fn all_time_outside_range_returns_zero() {
        let below  = below_range();
        let events = vec![
            make_event(0,   below, below),
            make_event(100, below, below),
            make_event(200, below, below),
        ];
        let eff = compute_range_efficiency(&events, TICK_LOWER, TICK_UPPER);
        assert_eq!(eff, 0.0);
    }

    #[test]
    fn half_time_in_range_returns_half() {
        // Slots 0-100: price exits range (after = below)
        // Slots 100-200: price re-enters range (after = mid)
        let mid   = range_mid();
        let below = below_range();

        let events = vec![
            make_event(0,   mid,   below), // after this: outside range for slots 0→100
            make_event(100, below, mid),   // after this: inside range for slots 100→200
            make_event(200, mid,   mid),   // endpoint
        ];

        let eff = compute_range_efficiency(&events, TICK_LOWER, TICK_UPPER);
        // slot gap 0-100 = 100 slots outside
        // slot gap 100-200 = 100 slots inside
        // efficiency = 100/200 = 0.5
        assert!(
            (eff - 0.5).abs() < 1e-9,
            "Expected 0.5, got {}",
            eff
        );
    }

    #[test]
    fn price_exits_and_re_enters_range() {
        let mid   = range_mid();
        let above = above_range();

        // 400 total slots
        // 0-100:   in range  (100 slots)
        // 100-300: outside   (200 slots)
        // 300-400: in range  (100 slots)
        let events = vec![
            make_event(0,   mid,   mid),   // after: in range for 0→100
            make_event(100, mid,   above), // after: outside for 100→300
            make_event(300, above, mid),   // after: in range for 300→400
            make_event(400, mid,   mid),   // endpoint
        ];

        let eff = compute_range_efficiency(&events, TICK_LOWER, TICK_UPPER);
        // in-range: 100 + 100 = 200 out of 400 total = 0.5
        assert!(
            (eff - 0.5).abs() < 1e-9,
            "Expected 0.5, got {}",
            eff
        );
    }

    #[test]
    fn efficiency_is_always_between_zero_and_one() {
        let mid   = range_mid();
        let below = below_range();
        let above = above_range();

        let events = vec![
            make_event(0,   mid,   below),
            make_event(50,  below, above),
            make_event(75,  above, mid),
            make_event(100, mid,   below),
            make_event(200, below, mid),
        ];

        let eff = compute_range_efficiency(&events, TICK_LOWER, TICK_UPPER);
        assert!(eff >= 0.0 && eff <= 1.0, "Efficiency {} out of [0,1]", eff);
    }

    #[test]
    fn weighted_by_slot_gaps() {
        // Long period in range, short period outside
        let mid   = range_mid();
        let below = below_range();

        // 0-900: in range (900 slots)
        // 900-1000: outside (100 slots)
        let events = vec![
            make_event(0,    mid,   mid),   // in range for 0→900
            make_event(900,  mid,   below), // outside for 900→1000
            make_event(1000, below, mid),   // endpoint
        ];

        let eff = compute_range_efficiency(&events, TICK_LOWER, TICK_UPPER);
        // 900/1000 = 0.9
        assert!(
            (eff - 0.9).abs() < 1e-9,
            "Expected 0.9, got {}",
            eff
        );
    }

    #[test]
    fn same_slot_for_all_events_returns_zero() {
        let mid    = range_mid();
        let events = vec![
            make_event(100, mid, mid),
            make_event(100, mid, mid),
        ];
        let eff = compute_range_efficiency(&events, TICK_LOWER, TICK_UPPER);
        assert_eq!(eff, 0.0);
    }
}