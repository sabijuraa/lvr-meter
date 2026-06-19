use crate::engine::optimizer::params::ParameterSet;
use crate::parser::types::SwapEvent;

/// Simulate the projected fee-to-LVR ratio for a given parameter set.
///
/// Based on Milionis et al. (2022) closed-form LVR formula and
/// log-normal price assumptions for fee income estimation.
///
/// Returns the projected fees / LVR ratio. Values > 1.0 indicate
/// the position would have been profitable under these parameters.
pub fn simulate_ratio(
    params:          &ParameterSet,
    volatility:      f64,
    observed_events: &[SwapEvent],
) -> f64 {
    if volatility <= 0.0 || observed_events.is_empty() {
        return 0.0;
    }

    let fee_rate        = params.fee_tier.as_decimal();
    let range_width_pct = params.range_width.as_decimal();

    let range_efficiency = estimate_range_efficiency(range_width_pct, volatility);
    let avg_trade_size   = estimate_avg_trade_size(observed_events);
    let event_count      = observed_events.len() as f64;

    let projected_fees = fee_rate * avg_trade_size * range_efficiency * event_count;
    let projected_lvr  = estimate_lvr(volatility, fee_rate, avg_trade_size, event_count);

    if projected_lvr <= 0.0 {
        return f64::INFINITY;
    }

    projected_fees / projected_lvr
}

/// Estimate what fraction of price movements fall within ±W% range
/// under log-normal assumptions.
///
/// Formula: erf(W / (σ * sqrt(2)))
/// where W is range half-width as decimal, σ is annualized volatility.
///
/// We scale σ to per-event volatility using the observed event frequency.
fn estimate_range_efficiency(range_width_decimal: f64, annualized_vol: f64) -> f64 {
    if annualized_vol <= 0.0 {
        return 1.0;
    }

    // Convert range width to number of standard deviations
    // range_width_decimal is the half-width (e.g. 0.08 for ±8%)
    let z = range_width_decimal / (annualized_vol / 2.0_f64.sqrt());

    erf(z)
}

/// Gauss error function approximation (Abramowitz & Stegun 7.1.26)
/// Maximum error: 1.5e-7
fn erf(x: f64) -> f64 {
    if x < 0.0 {
        return -erf(-x);
    }

    let t  = 1.0 / (1.0 + 0.3275911 * x);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;

    let poly = 0.254829592  * t
             - 0.284496736  * t2
             + 1.421413741  * t3
             - 1.453152027  * t4
             + 1.061405429  * t5;

    1.0 - poly * (-x * x).exp()
}

/// Estimate average trade size in USD from observed events
fn estimate_avg_trade_size(events: &[SwapEvent]) -> f64 {
    if events.is_empty() {
        return 0.0;
    }

    // Approximate trade size from price impact and liquidity
    // |ΔP| * liquidity_usd / price ≈ trade size
    let total: f64 = events
        .iter()
        .map(|e| {
            let price_impact = (e.price_after - e.price_before).abs();
            let liquidity_usd = e.active_liquidity as f64 / 1e18 * e.price_before;
            price_impact * liquidity_usd / e.price_before.max(1e-9)
        })
        .sum();

    total / events.len() as f64
}

/// Estimate LVR using Milionis formula:
///   LVR ≈ σ² * L / (8 * fee_rate) per unit time
///   Total LVR ≈ (σ² / (8 * fee_rate)) * avg_trade_size * event_count
fn estimate_lvr(
    volatility:     f64,
    fee_rate:       f64,
    avg_trade_size: f64,
    event_count:    f64,
) -> f64 {
    if fee_rate <= 0.0 {
        return f64::INFINITY;
    }

    // Per-event LVR approximation
    // σ here is per-event vol, approximated from annualized / sqrt(events_per_year)
    let per_event_vol = volatility / (365.0 * 24.0 * 3600.0 / 400.0_f64).sqrt();
    let per_event_lvr = (per_event_vol.powi(2) / (8.0 * fee_rate)) * avg_trade_size;

    per_event_lvr * event_count
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::optimizer::params::{FeeTier, ParameterSet, RangeWidth};
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
            active_liquidity:  (1_000_000.0 / price_before * 1e18) as u128,
            fee_rate:          25,
            direction:         SwapDirection::ZeroForOne,
        }
    }

    fn make_events(n: usize) -> Vec<SwapEvent> {
        (0..n).map(|_| make_event(150.0, 150.5)).collect()
    }

    fn params(bps: u16, pct: f64) -> ParameterSet {
        ParameterSet::new(FeeTier { basis_points: bps }, RangeWidth::new(pct))
    }

    #[test]
    fn ratio_increases_as_fee_tier_increases() {
        let events = make_events(100);
        let vol    = 0.8; // 80% annualized

        let low_fee  = simulate_ratio(&params(1,  8.0), vol, &events);
        let mid_fee  = simulate_ratio(&params(25, 8.0), vol, &events);
        let high_fee = simulate_ratio(&params(100, 8.0), vol, &events);

        assert!(
            low_fee < mid_fee,
            "ratio should increase with fee: {} < {}",
            low_fee, mid_fee
        );
        assert!(
            mid_fee < high_fee,
            "ratio should increase with fee: {} < {}",
            mid_fee, high_fee
        );
    }

    #[test]
    fn ratio_increases_as_range_width_increases_at_high_vol() {
        let events = make_events(100);
        let vol    = 1.5; // 150% annualized — high volatility

        let narrow = simulate_ratio(&params(25, 2.0),  vol, &events);
        let mid    = simulate_ratio(&params(25, 8.0),  vol, &events);
        let wide   = simulate_ratio(&params(25, 20.0), vol, &events);

        assert!(
            narrow <= mid,
            "wider range should have higher ratio at high vol: {} <= {}",
            narrow, mid
        );
        assert!(
            mid <= wide,
            "wider range should have higher ratio at high vol: {} <= {}",
            mid, wide
        );
    }

    #[test]
    fn zero_volatility_returns_zero() {
        let events = make_events(10);
        let ratio  = simulate_ratio(&params(25, 8.0), 0.0, &events);
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn empty_events_returns_zero() {
        let ratio = simulate_ratio(&params(25, 8.0), 0.8, &[]);
        assert_eq!(ratio, 0.0);
    }

    #[test]
    fn ratio_is_non_negative() {
        let events = make_events(50);
        for &vol in &[0.1, 0.5, 1.0, 2.0] {
            for &bps in &[1u16, 5, 25, 100] {
                for &pct in &[1.0f64, 5.0, 10.0, 25.0] {
                    let ratio = simulate_ratio(&params(bps, pct), vol, &events);
                    assert!(
                        ratio >= 0.0,
                        "Negative ratio {} for bps={} pct={} vol={}",
                        ratio, bps, pct, vol
                    );
                }
            }
        }
    }

    #[test]
    fn erf_known_values() {
        assert!((erf(0.0) - 0.0).abs()   < 1e-6);
        assert!((erf(1.0) - 0.8427).abs() < 1e-3);
        assert!((erf(2.0) - 0.9953).abs() < 1e-3);
        assert!((erf(-1.0) + 0.8427).abs() < 1e-3);
    }

    #[test]
    fn range_efficiency_increases_with_range_width() {
        let vol    = 0.8;
        let narrow = estimate_range_efficiency(0.02, vol);
        let wide   = estimate_range_efficiency(0.15, vol);
        assert!(
            narrow < wide,
            "wider range should capture more vol: {} < {}",
            narrow, wide
        );
    }

    #[test]
    fn range_efficiency_bounded_zero_to_one() {
        for &w in &[0.01f64, 0.05, 0.10, 0.25] {
            let eff = estimate_range_efficiency(w, 0.8);
            assert!(eff >= 0.0 && eff <= 1.0, "efficiency {} out of bounds", eff);
        }
    }
}