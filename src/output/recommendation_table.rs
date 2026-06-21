use colored::Colorize;
use tabled::{Table, Tabled};

use crate::engine::optimizer::search::{ConfidenceLevel, OptimizerResult};
use crate::engine::regime::RegimeResult;

#[derive(Tabled)]
struct RecommendationRow {
    #[tabled(rename = "Parameter")]
    parameter: String,

    #[tabled(rename = "Value")]
    value: String,
}

pub fn print_recommendation_table(
    optimizer: &OptimizerResult,
    regime:    &RegimeResult,
    current_price: f64,
) {
    println!("\n=== Section 2 — Parameter Recommendation ===\n");

    let rows = vec![
        RecommendationRow {
            parameter: "Recommended Fee Tier".to_string(),
            value:     optimizer.optimal_params.fee_tier.to_string(),
        },
        RecommendationRow {
            parameter: "Recommended Range Width".to_string(),
            value:     optimizer.optimal_params.range_width.to_string(),
        },
        RecommendationRow {
            parameter: "Projected Fee-to-LVR Ratio".to_string(),
            value:     format!("{:.3}", optimizer.projected_ratio),
        },
        RecommendationRow {
            parameter: "Confidence".to_string(),
            value:     colorize_confidence(&optimizer.confidence),
        },
        RecommendationRow {
            parameter: "Basis — Volatility".to_string(),
            value:     format!("{:.1}% annualized", regime.annualized_volatility * 100.0),
        },
        RecommendationRow {
            parameter: "Basis — Regime".to_string(),
            value:     regime.regime.to_string(),
        },
    ];

    println!("{}", Table::new(&rows));
    println!();

    print_action_line(optimizer, current_price);

    if let Some(runner_up) = &optimizer.runner_up {
        println!(
            "\n  Runner-up: {} — also within 5% of optimal.",
            runner_up
        );
        println!(
            "  Wide recommendation plateau — confidence adjusted upward."
        );
    }

    println!();
}

fn print_action_line(optimizer: &OptimizerResult, current_price: f64) {
    let action = format!(
        "  → Center a {} range around the current price of ${:.2} in the {} fee tier.",
        optimizer.optimal_params.range_width,
        current_price,
        optimizer.optimal_params.fee_tier,
    );

    let colored_action = match optimizer.confidence {
        ConfidenceLevel::High   => action.green().bold().to_string(),
        ConfidenceLevel::Medium => action.yellow().to_string(),
        ConfidenceLevel::Low    => action.dimmed().to_string(),
    };

    println!("{}", colored_action);
}

fn colorize_confidence(level: &ConfidenceLevel) -> String {
    match level {
        ConfidenceLevel::High   => level.to_string().green().bold().to_string(),
        ConfidenceLevel::Medium => level.to_string().yellow().to_string(),
        ConfidenceLevel::Low    => level.to_string().red().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::optimizer::params::{FeeTier, ParameterSet, RangeWidth};
    use crate::engine::optimizer::search::ConfidenceLevel;
    use crate::engine::regime::{Regime, RegimeResult};

    fn make_optimizer_result(
        bps:        u16,
        pct:        f64,
        ratio:      f64,
        confidence: ConfidenceLevel,
    ) -> OptimizerResult {
        OptimizerResult {
            optimal_params:  ParameterSet::new(
                FeeTier { basis_points: bps },
                RangeWidth::new(pct),
            ),
            projected_ratio: ratio,
            confidence,
            runner_up:       None,
        }
    }

    fn make_regime(vol: f64) -> RegimeResult {
        RegimeResult {
            annualized_volatility: vol,
            regime:                Regime::MeanReverting,
            trending_fraction:     0.3,
        }
    }

    #[test]
    fn does_not_panic_on_valid_input() {
        let opt    = make_optimizer_result(25, 8.0, 1.35, ConfidenceLevel::High);
        let regime = make_regime(0.8);
        print_recommendation_table(&opt, &regime, 150.0);
    }

    #[test]
    fn does_not_panic_with_runner_up() {
        let mut opt = make_optimizer_result(25, 8.0, 1.35, ConfidenceLevel::High);
        opt.runner_up = Some(ParameterSet::new(
            FeeTier { basis_points: 25 },
            RangeWidth::new(9.0),
        ));
        let regime = make_regime(0.8);
        print_recommendation_table(&opt, &regime, 150.0);
    }

    #[test]
    fn confidence_colorize_does_not_panic() {
        colorize_confidence(&ConfidenceLevel::High);
        colorize_confidence(&ConfidenceLevel::Medium);
        colorize_confidence(&ConfidenceLevel::Low);
    }

    #[test]
    fn high_confidence_renders_correctly() {
        let opt    = make_optimizer_result(100, 20.0, 1.5, ConfidenceLevel::High);
        let regime = make_regime(1.5);
        // Should not panic even with high vol and high fee tier
        print_recommendation_table(&opt, &regime, 200.0);
    }

    #[test]
    fn low_confidence_renders_correctly() {
        let opt    = make_optimizer_result(1, 1.0, 0.5, ConfidenceLevel::Low);
        let regime = make_regime(0.1);
        print_recommendation_table(&opt, &regime, 50.0);
    }
}