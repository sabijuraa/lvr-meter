//! Property-based tests for the optimizer using proptest.
//!
//! These encode domain knowledge as invariants that must hold
//! for any valid input in the specified ranges.

use lvr_meter::engine::optimizer::params::{FeeTier, ParameterSet};
use lvr_meter::engine::optimizer::search::run_optimizer;
use lvr_meter::engine::regime::{Regime, RegimeResult};
use lvr_meter::parser::types::{SwapDirection, SwapEvent};
use proptest::prelude::*;
use solana_sdk::pubkey::Pubkey;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_regime(vol: f64) -> RegimeResult {
    let regime = if vol > 1.5 {
        Regime::Volatile
    } else if vol < 0.3 {
        Regime::MeanReverting
    } else {
        Regime::MeanReverting
    };

    RegimeResult {
        annualized_volatility: vol,
        regime,
        trending_fraction:     0.3,
    }
}

fn make_events(n: usize, price: f64) -> Vec<SwapEvent> {
    (0..n)
        .map(|i| SwapEvent {
            slot:              i as u64 * 100,
            timestamp:         1_735_689_600 + i as i64 * 400,
            pool:              Pubkey::new_unique(),
            price_before:      price,
            price_after:       price * 1.001,
            sqrt_price_before: 0,
            sqrt_price_after:  0,
            active_liquidity:  (1_000_000.0 / price * 1e18) as u128,
            fee_rate:          25,
            direction:         SwapDirection::OneForZero,
        })
        .collect()
}

// ── Property Tests ────────────────────────────────────────────────────────────

proptest! {
    /// For any valid volatility in [0.1, 3.0], the optimizer always returns
    /// a ParameterSet that exists in the search space.
    #[test]
    fn optimizer_always_returns_valid_parameter_set(
        vol   in 0.1f64..3.0f64,
        n     in 10usize..100usize,
        price in 10.0f64..1000.0f64,
    ) {
        let regime = make_regime(vol);
        let events = make_events(n, price);
        let result = run_optimizer(&regime, &events);

        let search_space = ParameterSet::all();
        prop_assert!(
            search_space.contains(&result.optimal_params),
            "Result {:?} not in search space",
            result.optimal_params
        );
    }

    /// For any valid volatility, projected_ratio is always non-negative.
    #[test]
    fn projected_ratio_is_always_non_negative(
        vol   in 0.1f64..3.0f64,
        n     in 10usize..50usize,
    ) {
        let regime = make_regime(vol);
        let events = make_events(n, 150.0);
        let result = run_optimizer(&regime, &events);

        prop_assert!(
            result.projected_ratio >= 0.0,
            "Negative projected_ratio: {}",
            result.projected_ratio
        );
    }

    /// For high volatility (above 1.5 annualized), the optimal range width
    /// is always above 5%. Narrow ranges cannot survive high volatility —
    /// the price exits too frequently and fees collected drop to near zero.
    #[test]
    fn high_volatility_recommends_wide_range(
        vol in 1.5f64..3.0f64,
        n   in 20usize..100usize,
    ) {
        let regime = make_regime(vol);
        let events = make_events(n, 150.0);
        let result = run_optimizer(&regime, &events);

        prop_assert!(
            result.optimal_params.range_width.percent > 5.0,
            "High vol {:.2} recommended narrow range {:.1}% — expected > 5%",
            vol,
            result.optimal_params.range_width.percent
        );
    }

    /// For low volatility (below 0.3 annualized), the optimal fee tier
    /// is never the highest tier (100 bps). High fees at low volatility
    /// repel traders and reduce volume — the fee income does not compensate.
    #[test]
    fn low_volatility_does_not_recommend_highest_fee_tier(
        vol in 0.1f64..0.3f64,
        n   in 20usize..100usize,
    ) {
        let regime = make_regime(vol);
        let events = make_events(n, 150.0);
        let result = run_optimizer(&regime, &events);

        prop_assert_ne!(
            result.optimal_params.fee_tier,
            FeeTier::ONE_HUNDRED,
            "Low vol {:.2} recommended highest fee tier 100 bps — not expected",
            vol,
        );
    }

    /// The runner-up, if present, must be a different ParameterSet from optimal.
    #[test]
    fn runner_up_is_always_different_from_optimal(
        vol in 0.1f64..3.0f64,
        n   in 10usize..50usize,
    ) {
        let regime = make_regime(vol);
        let events = make_events(n, 150.0);
        let result = run_optimizer(&regime, &events);

        if let Some(runner_up) = &result.runner_up {
            prop_assert_ne!(
                runner_up,
                &result.optimal_params,
                "Runner-up is identical to optimal"
            );
        }
    }

    /// Projected ratio must be finite (no NaN or infinity leaking out).
    #[test]
    fn projected_ratio_is_always_finite(
        vol   in 0.1f64..3.0f64,
        n     in 10usize..50usize,
        price in 1.0f64..10000.0f64,
    ) {
        let regime = make_regime(vol);
        let events = make_events(n, price);
        let result = run_optimizer(&regime, &events);

        prop_assert!(
            result.projected_ratio.is_finite(),
            "projected_ratio is not finite: {}",
            result.projected_ratio
        );
    }
}