use anyhow::Result;
use serde::Serialize;

use crate::engine::analysis::PositionAnalysis;
use crate::engine::optimizer::search::OptimizerResult;

#[derive(Debug, Serialize)]
pub struct LvrResultJson {
    pub total_lvr_usd:      f64,
    pub event_count:        usize,
    pub largest_single_lvr: f64,
    pub lvr_by_day:         Vec<(String, f64)>,
}

#[derive(Debug, Serialize)]
pub struct FeeResultJson {
    pub fees_token_0: u64,
    pub fees_token_1: u64,
    pub fees_usd:     f64,
}

#[derive(Debug, Serialize)]
pub struct VerdictJson {
    pub ratio:       f64,
    pub label:       String,
    pub net_pnl_usd: f64,
}

#[derive(Debug, Serialize)]
pub struct RegimeJson {
    pub annualized_volatility: f64,
    pub regime:                String,
    pub trending_fraction:     f64,
}

#[derive(Debug, Serialize)]
pub struct PositionAnalysisJson {
    pub lvr:              LvrResultJson,
    pub fees:             FeeResultJson,
    pub verdict:          VerdictJson,
    pub range_efficiency: f64,
    pub regime:           RegimeJson,
}

#[derive(Debug, Serialize)]
pub struct OptimizerResultJson {
    pub fee_tier_bps:    u16,
    pub fee_tier_pct:    f64,
    pub range_width_pct: f64,
    pub projected_ratio: f64,
    pub confidence:      String,
    pub runner_up_fee_tier_bps:    Option<u16>,
    pub runner_up_range_width_pct: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct JsonOutput {
    pub analyses:  Vec<PositionAnalysisJson>,
    pub optimizer: OptimizerResultJson,
}

impl PositionAnalysisJson {
    pub fn from_analysis(analysis: &PositionAnalysis) -> Self {
        Self {
            lvr: LvrResultJson {
                total_lvr_usd:      analysis.lvr.total_lvr_usd,
                event_count:        analysis.lvr.event_count,
                largest_single_lvr: analysis.lvr.largest_single_lvr,
                lvr_by_day:         analysis
                    .lvr
                    .lvr_by_day
                    .iter()
                    .map(|(d, v)| (d.to_string(), *v))
                    .collect(),
            },
            fees: FeeResultJson {
                fees_token_0: analysis.fees.fees_token_0,
                fees_token_1: analysis.fees.fees_token_1,
                fees_usd:     analysis.fees.fees_usd,
            },
            verdict: VerdictJson {
                ratio:       analysis.verdict.ratio,
                label:       analysis.verdict.label.to_string(),
                net_pnl_usd: analysis.verdict.net_pnl_usd,
            },
            range_efficiency: analysis.range_efficiency,
            regime: RegimeJson {
                annualized_volatility: analysis.regime.annualized_volatility,
                regime:                analysis.regime.regime.to_string(),
                trending_fraction:     analysis.regime.trending_fraction,
            },
        }
    }
}

impl OptimizerResultJson {
    pub fn from_result(result: &OptimizerResult) -> Self {
        Self {
            fee_tier_bps:    result.optimal_params.fee_tier.basis_points,
            fee_tier_pct:    result.optimal_params.fee_tier.as_decimal() * 100.0,
            range_width_pct: result.optimal_params.range_width.percent,
            projected_ratio: result.projected_ratio,
            confidence:      result.confidence.to_string(),
            runner_up_fee_tier_bps: result
                .runner_up
                .as_ref()
                .map(|r| r.fee_tier.basis_points),
            runner_up_range_width_pct: result
                .runner_up
                .as_ref()
                .map(|r| r.range_width.percent),
        }
    }
}

pub fn print_json_output(
    analyses:  &[PositionAnalysis],
    optimizer: &OptimizerResult,
) -> Result<()> {
    let output = JsonOutput {
        analyses:  analyses
            .iter()
            .map(PositionAnalysisJson::from_analysis)
            .collect(),
        optimizer: OptimizerResultJson::from_result(optimizer),
    };

    let json = serde_json::to_string_pretty(&output)?;
    println!("{}", json);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::accumulator::LvrResult;
    use crate::engine::fees::FeeResult;
    use crate::engine::optimizer::params::{FeeTier, ParameterSet, RangeWidth};
    use crate::engine::optimizer::search::ConfidenceLevel;
    use crate::engine::regime::{Regime, RegimeResult};
    use crate::engine::verdict::{Verdict, VerdictLabel};

    fn make_analysis() -> PositionAnalysis {
        PositionAnalysis {
            lvr: LvrResult {
                total_lvr_usd:      100.0,
                event_count:        10,
                largest_single_lvr: 15.0,
                lvr_by_day:         vec![],
            },
            fees: FeeResult {
                fees_token_0: 500,
                fees_token_1: 300,
                fees_usd:     150.0,
            },
            verdict: Verdict {
                ratio:       1.5,
                label:       VerdictLabel::Profitable,
                net_pnl_usd: 50.0,
            },
            range_efficiency: 0.75,
            regime: RegimeResult {
                annualized_volatility: 0.8,
                regime:                Regime::MeanReverting,
                trending_fraction:     0.3,
            },
        }
    }

    fn make_optimizer() -> OptimizerResult {
        OptimizerResult {
            optimal_params:  ParameterSet::new(
                FeeTier { basis_points: 25 },
                RangeWidth::new(8.0),
            ),
            projected_ratio: 1.34,
            confidence:      ConfidenceLevel::Medium,
            runner_up:       None,
        }
    }

    #[test]
    fn serializes_to_valid_json() {
        let analysis  = make_analysis();
        let optimizer = make_optimizer();
        let output    = JsonOutput {
            analyses:  vec![PositionAnalysisJson::from_analysis(&analysis)],
            optimizer: OptimizerResultJson::from_result(&optimizer),
        };
        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(!json.is_empty());
        assert!(json.contains("total_lvr_usd"));
        assert!(json.contains("fees_usd"));
        assert!(json.contains("projected_ratio"));
    }

    #[test]
    fn deserializes_back_from_json() {
        let analysis  = make_analysis();
        let optimizer = make_optimizer();
        let output    = JsonOutput {
            analyses:  vec![PositionAnalysisJson::from_analysis(&analysis)],
            optimizer: OptimizerResultJson::from_result(&optimizer),
        };

        let json     = serde_json::to_string_pretty(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Verify key fields round-trip correctly
        assert_eq!(
            parsed["analyses"][0]["lvr"]["total_lvr_usd"],
            100.0
        );
        assert_eq!(
            parsed["analyses"][0]["fees"]["fees_usd"],
            150.0
        );
        assert_eq!(
            parsed["analyses"][0]["verdict"]["ratio"],
            1.5
        );
        assert_eq!(
            parsed["optimizer"]["fee_tier_bps"],
            25
        );
        assert_eq!(
            parsed["optimizer"]["range_width_pct"],
            8.0
        );
        assert_eq!(
            parsed["optimizer"]["confidence"],
            "medium"
        );
    }

    #[test]
    fn verdict_label_serializes_as_string() {
        let analysis = make_analysis();
        let json     = PositionAnalysisJson::from_analysis(&analysis);
        let s        = serde_json::to_string(&json).unwrap();
        assert!(s.contains("PROFITABLE"));
    }

    #[test]
    fn runner_up_none_serializes_as_null() {
        let optimizer = make_optimizer();
        let json      = OptimizerResultJson::from_result(&optimizer);
        let s         = serde_json::to_string(&json).unwrap();
        assert!(s.contains("null"));
    }

    #[test]
    fn runner_up_some_serializes_correctly() {
        let mut optimizer = make_optimizer();
        optimizer.runner_up = Some(ParameterSet::new(
            FeeTier { basis_points: 25 },
            RangeWidth::new(9.0),
        ));
        let json = OptimizerResultJson::from_result(&optimizer);
        let s    = serde_json::to_string(&json).unwrap();
        assert!(s.contains("25"));
        assert!(s.contains("9.0"));
    }

    #[test]
    fn multiple_analyses_serialize() {
        let analysis  = make_analysis();
        let optimizer = make_optimizer();
        let output    = JsonOutput {
            analyses:  vec![
                PositionAnalysisJson::from_analysis(&analysis),
                PositionAnalysisJson::from_analysis(&analysis),
            ],
            optimizer: OptimizerResultJson::from_result(&optimizer),
        };
        let json   = serde_json::to_string_pretty(&output).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["analyses"].as_array().unwrap().len(), 2);
    }
}