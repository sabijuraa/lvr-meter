use crate::engine::optimizer::params::ParameterSet;
use crate::engine::optimizer::simulator::simulate_ratio;
use crate::engine::regime::RegimeResult;
use crate::parser::types::SwapEvent;

#[derive(Debug, Clone, PartialEq)]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfidenceLevel::High   => write!(f, "high"),
            ConfidenceLevel::Medium => write!(f, "medium"),
            ConfidenceLevel::Low    => write!(f, "low"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptimizerResult {
    pub optimal_params:  ParameterSet,
    pub projected_ratio: f64,
    pub confidence:      ConfidenceLevel,
    pub runner_up:       Option<ParameterSet>,
}

impl OptimizerResult {
    pub fn recommendation_line(&self) -> String {
        format!(
            "Recommended: {} fee tier, {} range. Projected ratio: {:.2}. Confidence: {}.",
            self.optimal_params.fee_tier,
            self.optimal_params.range_width,
            self.projected_ratio,
            self.confidence,
        )
    }
}

const HIGH_CONFIDENCE_RATIO:   f64 = 1.3;
const MEDIUM_CONFIDENCE_RATIO: f64 = 1.1;
const RUNNER_UP_THRESHOLD:     f64 = 0.05; // within 5% of optimal

pub fn run_optimizer(
    regime: &RegimeResult,
    events: &[SwapEvent],
) -> OptimizerResult {
    let search_space = ParameterSet::all();
    let volatility   = regime.annualized_volatility;

    // Score every parameter combination
    let mut scored: Vec<(ParameterSet, f64)> = search_space
        .into_iter()
        .map(|params| {
            let ratio = simulate_ratio(&params, volatility, events);
            (params, ratio)
        })
        .collect();

    // Sort descending by projected ratio
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let (optimal_params, projected_ratio) = scored
        .first()
        .cloned()
        .unwrap_or_else(|| (ParameterSet::all().remove(0), 0.0));

    let runner_up = scored
        .get(1)
        .filter(|(_, ratio)| {
            projected_ratio > 0.0
                && (projected_ratio - ratio).abs() / projected_ratio <= RUNNER_UP_THRESHOLD
        })
        .map(|(params, _)| params.clone());

    let confidence = compute_confidence(
        projected_ratio,
        runner_up.is_some(),
    );

    OptimizerResult {
        optimal_params,
        projected_ratio,
        confidence,
        runner_up,
    }
}

fn compute_confidence(
    projected_ratio: f64,
    has_close_runner_up: bool,
) -> ConfidenceLevel {
    if projected_ratio > HIGH_CONFIDENCE_RATIO {
        // Wide plateau (close runner-up) increases confidence further
        if has_close_runner_up {
            ConfidenceLevel::High
        } else {
            ConfidenceLevel::High
        }
    } else if projected_ratio >= MEDIUM_CONFIDENCE_RATIO {
        ConfidenceLevel::Medium
    } else {
        ConfidenceLevel::Low
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::regime::{Regime, RegimeResult};
    use crate::engine::optimizer::params::{FeeTier, RangeWidth};
    use crate::parser::types::{SwapDirection, SwapEvent};
    use solana_sdk::pubkey::Pubkey;

    fn make_regime(vol: f64, regime: Regime) -> RegimeResult {
        RegimeResult {
            annualized_volatility: vol,
            regime,
            trending_fraction:     0.3,
        }
    }

    fn make_events(n: usize, price: f64) -> Vec<SwapEvent> {
        (0..n)
            .map(|_| SwapEvent {
                slot:              1,
                timestamp:         0,
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

    #[test]
    fn returns_a_result_for_any_input() {
        let regime = make_regime(0.8, Regime::MeanReverting);
        let events = make_events(50, 150.0);
        let result = run_optimizer(&regime, &events);

        assert!(result.projected_ratio >= 0.0);
        assert!(ParameterSet::all().contains(&result.optimal_params));
    }

    #[test]
    fn optimal_is_highest_scoring() {
        let regime = make_regime(0.8, Regime::MeanReverting);
        let events = make_events(50, 150.0);
        let result = run_optimizer(&regime, &events);

        // Every other parameter set should score <= optimal
        for params in ParameterSet::all() {
            let ratio = simulate_ratio(&params, 0.8, &events);
            assert!(
                ratio <= result.projected_ratio + 1e-9,
                "Found better ratio {} > optimal {}",
                ratio, result.projected_ratio
            );
        }
    }

    #[test]
    fn confidence_high_when_ratio_above_1_3() {
        let regime = make_regime(0.01, Regime::MeanReverting); // low vol → high ratio
        let events = make_events(100, 150.0);
        let result = run_optimizer(&regime, &events);

        if result.projected_ratio > HIGH_CONFIDENCE_RATIO {
            assert_eq!(result.confidence, ConfidenceLevel::High);
        }
    }

    #[test]
    fn confidence_low_when_ratio_below_1_1() {
        // Zero volatility → zero ratio → low confidence
        let regime = make_regime(0.0, Regime::MeanReverting);
        let events = make_events(10, 150.0);
        let result = run_optimizer(&regime, &events);

        assert_eq!(result.confidence, ConfidenceLevel::Low);
    }

    #[test]
    fn runner_up_is_different_from_optimal() {
        let regime = make_regime(0.8, Regime::MeanReverting);
        let events = make_events(50, 150.0);
        let result = run_optimizer(&regime, &events);

        if let Some(runner_up) = &result.runner_up {
            assert_ne!(runner_up, &result.optimal_params);
        }
    }

    #[test]
    fn recommendation_line_contains_required_fields() {
        let regime = make_regime(0.8, Regime::MeanReverting);
        let events = make_events(50, 150.0);
        let result = run_optimizer(&regime, &events);
        let line   = result.recommendation_line();

        assert!(line.contains("Recommended:"));
        assert!(line.contains("fee tier"));
        assert!(line.contains("range"));
        assert!(line.contains("Projected ratio:"));
        assert!(line.contains("Confidence:"));
    }

    #[test]
    fn confidence_display() {
        assert_eq!(ConfidenceLevel::High.to_string(),   "high");
        assert_eq!(ConfidenceLevel::Medium.to_string(), "medium");
        assert_eq!(ConfidenceLevel::Low.to_string(),    "low");
    }

    #[test]
    fn empty_events_still_returns_result() {
        let regime = make_regime(0.8, Regime::Volatile);
        let result = run_optimizer(&regime, &[]);

        assert_eq!(result.projected_ratio, 0.0);
        assert_eq!(result.confidence, ConfidenceLevel::Low);
    }
}