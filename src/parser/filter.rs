use crate::parser::types::SwapEvent;

/// Convert a tick index to a price.
/// Formula: price = 1.0001^tick
pub fn tick_to_price(tick: i32) -> f64 {
    1.0001_f64.powi(tick)
}

/// Filter swap events to only those that affected a position's tick range.
///
/// A swap is included if the price movement overlaps with the range [tick_lower, tick_upper].
/// This includes:
///   - Swaps entirely within the range
///   - Swaps that enter the range (started outside, ended inside)
///   - Swaps that exit the range (started inside, ended outside)
///   - Swaps that cross the entire range
pub fn filter_swaps_to_position(
    events:     &[SwapEvent],
    tick_lower: i32,
    tick_upper: i32,
) -> Vec<SwapEvent> {
    let price_lower = tick_to_price(tick_lower);
    let price_upper = tick_to_price(tick_upper);

    events
        .iter()
        .filter(|e| overlaps_range(e.price_before, e.price_after, price_lower, price_upper))
        .cloned()
        .collect()
}

/// Returns true if the price movement from `before` to `after` overlaps
/// with the range [range_low, range_high].
///
/// Overlap occurs unless the entire movement is strictly outside the range
/// on one side.
fn overlaps_range(
    before:     f64,
    after:      f64,
    range_low:  f64,
    range_high: f64,
) -> bool {
    let move_low  = before.min(after);
    let move_high = before.max(after);

    // No overlap if movement is entirely below or entirely above the range
    !(move_high < range_low || move_low > range_high)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::types::{SwapDirection, SwapEvent};
    use solana_sdk::pubkey::Pubkey;

    fn make_event(price_before: f64, price_after: f64) -> SwapEvent {
        SwapEvent {
            slot:              1,
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

    // tick range [-100, 100] → prices [0.99005, 1.01005]
    const TICK_LOWER: i32 = -100;
    const TICK_UPPER: i32 =  100;

    #[test]
    fn tick_to_price_known_values() {
        assert!((tick_to_price(0)    - 1.0).abs()    < 1e-9);
        assert!((tick_to_price(1)    - 1.0001).abs() < 1e-9);
        assert!((tick_to_price(-1)   - (1.0 / 1.0001)).abs() < 1e-9);
        assert!((tick_to_price(10000) - 2.7181).abs() < 0.001);
    }

    #[test]
    fn swap_entirely_inside_range_is_included() {
        let p_low  = tick_to_price(TICK_LOWER);
        let p_high = tick_to_price(TICK_UPPER);
        let mid    = (p_low + p_high) / 2.0;
        let events = vec![make_event(mid - 0.0001, mid + 0.0001)];

        let filtered = filter_swaps_to_position(&events, TICK_LOWER, TICK_UPPER);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn swap_entirely_below_range_is_excluded() {
        let p_low  = tick_to_price(TICK_LOWER);
        let events = vec![make_event(p_low * 0.5, p_low * 0.6)];

        let filtered = filter_swaps_to_position(&events, TICK_LOWER, TICK_UPPER);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn swap_entirely_above_range_is_excluded() {
        let p_high = tick_to_price(TICK_UPPER);
        let events = vec![make_event(p_high * 1.5, p_high * 2.0)];

        let filtered = filter_swaps_to_position(&events, TICK_LOWER, TICK_UPPER);
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn swap_entering_range_from_below_is_included() {
        let p_low  = tick_to_price(TICK_LOWER);
        let p_high = tick_to_price(TICK_UPPER);
        let mid    = (p_low + p_high) / 2.0;
        // starts below range, ends inside
        let events = vec![make_event(p_low * 0.5, mid)];

        let filtered = filter_swaps_to_position(&events, TICK_LOWER, TICK_UPPER);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn swap_exiting_range_above_is_included() {
        let p_low  = tick_to_price(TICK_LOWER);
        let p_high = tick_to_price(TICK_UPPER);
        let mid    = (p_low + p_high) / 2.0;
        // starts inside, ends above
        let events = vec![make_event(mid, p_high * 1.5)];

        let filtered = filter_swaps_to_position(&events, TICK_LOWER, TICK_UPPER);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn swap_crossing_entire_range_is_included() {
        let p_low  = tick_to_price(TICK_LOWER);
        let p_high = tick_to_price(TICK_UPPER);
        // starts below, ends above — crosses the whole range
        let events = vec![make_event(p_low * 0.5, p_high * 2.0)];

        let filtered = filter_swaps_to_position(&events, TICK_LOWER, TICK_UPPER);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn mixed_events_filtered_correctly() {
        let p_low  = tick_to_price(TICK_LOWER);
        let p_high = tick_to_price(TICK_UPPER);
        let mid    = (p_low + p_high) / 2.0;

        let events = vec![
            make_event(mid - 0.0001, mid + 0.0001), // inside  → included
            make_event(p_low * 0.1,  p_low * 0.5),  // below   → excluded
            make_event(p_high * 1.5, p_high * 2.0), // above   → excluded
            make_event(p_low * 0.5,  mid),           // enters  → included
        ];

        let filtered = filter_swaps_to_position(&events, TICK_LOWER, TICK_UPPER);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn empty_events_returns_empty() {
        let filtered = filter_swaps_to_position(&[], TICK_LOWER, TICK_UPPER);
        assert_eq!(filtered.len(), 0);
    }
}