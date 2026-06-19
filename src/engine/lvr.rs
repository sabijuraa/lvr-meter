use crate::parser::types::SwapEvent;

/// Compute instantaneous LVR for a single swap event.
///
/// Formula (Milionis et al. 2022):
///   LVR = (ΔP)² / (8 * fee_rate) * L_usd
///
/// Where:
///   ΔP      = price_after - price_before
///   fee_rate = fee in decimal (e.g. 0.0025 for 25 bps)
///   L_usd   = liquidity value in USD at price_before
///
/// Liquidity in USD is approximated as:
///   L_usd = active_liquidity / 10^18 * price_before
///
/// The active_liquidity is in raw Solana units (u128). We scale by 1e18
/// as a normalizing constant — the absolute scale cancels in the ratio
/// when comparing LVR to fees, which matters more than the absolute value.
pub fn compute_instantaneous_lvr(event: &SwapEvent) -> f64 {
    if event.fee_rate == 0 || event.active_liquidity == 0 {
        return 0.0;
    }

    let delta_price   = event.price_after - event.price_before;
    let fee_rate      = event.fee_rate_decimal();
    let liquidity_usd = liquidity_to_usd(event.active_liquidity, event.price_before);

    (delta_price.powi(2) / (8.0 * fee_rate)) * liquidity_usd
}

/// Convert raw liquidity units to a USD-denominated value.
///
/// Raw liquidity is in Solana's internal units (sqrt(x*y) scaled).
/// We normalize by 1e18 to get a workable float, then multiply by
/// the current price to express in USD terms.
fn liquidity_to_usd(liquidity: u128, price: f64) -> f64 {
    const LIQUIDITY_SCALE: f64 = 1e18;
    (liquidity as f64 / LIQUIDITY_SCALE) * price
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::types::{SwapDirection, SwapEvent};
    use solana_sdk::pubkey::Pubkey;

    /// Build a SwapEvent with explicit liquidity in "USD units" for testing.
    /// We back-calculate raw liquidity from a desired USD value:
    ///   raw_liquidity = usd_value / price * 1e18
    fn make_event_with_usd_liquidity(
        price_before:   f64,
        price_after:    f64,
        fee_rate_bps:   u16,
        liquidity_usd:  f64,
    ) -> SwapEvent {
        let raw_liquidity = (liquidity_usd / price_before * 1e18) as u128;

        SwapEvent {
            slot:              1,
            timestamp:         0,
            pool:              Pubkey::new_unique(),
            price_before,
            price_after,
            sqrt_price_before: 0,
            sqrt_price_after:  0,
            active_liquidity:  raw_liquidity,
            fee_rate:          fee_rate_bps,
            direction:         if price_after < price_before {
                SwapDirection::ZeroForOne
            } else {
                SwapDirection::OneForZero
            },
        }
    }

    #[test]
    fn hand_calculated_known_case() {
        // Price: 100.0 → 100.5  (ΔP = 0.5)
        // Fee:   25 bps = 0.0025
        // Liquidity: $1,000,000 USD
        //
        // LVR = (0.5)^2 / (8 * 0.0025) * 1_000_000
        //     = 0.25 / 0.02 * 1_000_000
        //     = 12.5 * 1_000_000
        //     = 12_500_000.0
        let event    = make_event_with_usd_liquidity(100.0, 100.5, 25, 1_000_000.0);
        let lvr      = compute_instantaneous_lvr(&event);
        let expected = 12_500_000.0_f64;

        let relative_error = ((lvr - expected) / expected).abs();
        assert!(
            relative_error < 1e-6,
            "LVR {} differs from expected {} by {:.8}",
            lvr, expected, relative_error
        );
    }

    #[test]
    fn zero_price_movement_gives_zero_lvr() {
        let event = make_event_with_usd_liquidity(100.0, 100.0, 25, 1_000_000.0);
        assert_eq!(compute_instantaneous_lvr(&event), 0.0);
    }

    #[test]
    fn zero_fee_rate_gives_zero_lvr() {
        let event = make_event_with_usd_liquidity(100.0, 101.0, 0, 1_000_000.0);
        assert_eq!(compute_instantaneous_lvr(&event), 0.0);
    }

    #[test]
    fn zero_liquidity_gives_zero_lvr() {
        let mut event = make_event_with_usd_liquidity(100.0, 101.0, 25, 1_000_000.0);
        event.active_liquidity = 0;
        assert_eq!(compute_instantaneous_lvr(&event), 0.0);
    }

    #[test]
    fn lvr_is_symmetric_for_equal_price_moves() {
        // LVR depends on (ΔP)^2 so up and down moves of equal magnitude
        // should produce equal LVR
        let up   = make_event_with_usd_liquidity(100.0, 101.0, 25, 1_000_000.0);
        let down = make_event_with_usd_liquidity(100.0,  99.0, 25, 1_000_000.0);

        let lvr_up   = compute_instantaneous_lvr(&up);
        let lvr_down = compute_instantaneous_lvr(&down);

        let diff = (lvr_up - lvr_down).abs();
        assert!(
            diff < 1.0,
            "LVR should be symmetric: up={} down={} diff={}",
            lvr_up, lvr_down, diff
        );
    }

    #[test]
    fn higher_fee_rate_produces_lower_lvr() {
        // Higher fee rate → less LVR extraction (arbitrageur waits for bigger moves)
        let low_fee  = make_event_with_usd_liquidity(100.0, 101.0,  5, 1_000_000.0);
        let high_fee = make_event_with_usd_liquidity(100.0, 101.0, 100, 1_000_000.0);

        assert!(
            compute_instantaneous_lvr(&low_fee) > compute_instantaneous_lvr(&high_fee),
            "Higher fee should produce lower LVR"
        );
    }

    #[test]
    fn larger_price_move_produces_higher_lvr() {
        let small_move = make_event_with_usd_liquidity(100.0, 100.5, 25, 1_000_000.0);
        let large_move = make_event_with_usd_liquidity(100.0, 102.0, 25, 1_000_000.0);

        assert!(
            compute_instantaneous_lvr(&large_move) > compute_instantaneous_lvr(&small_move),
            "Larger price move should produce higher LVR"
        );
    }
}