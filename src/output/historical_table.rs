use colored::Colorize;
use tabled::{Table, Tabled};

use crate::engine::analysis::PositionAnalysis;
use crate::engine::verdict::VerdictLabel;

#[derive(Tabled)]
struct AnalysisRow {
    #[tabled(rename = "Pool")]
    pool: String,

    #[tabled(rename = "Period")]
    period: String,

    #[tabled(rename = "Fees ($)")]
    fees_usd: String,

    #[tabled(rename = "LVR ($)")]
    lvr_usd: String,

    #[tabled(rename = "Ratio")]
    ratio: String,

    #[tabled(rename = "Efficiency")]
    efficiency: String,

    #[tabled(rename = "Regime")]
    regime: String,

    #[tabled(rename = "Verdict")]
    verdict: String,
}

pub struct AnalysisInput<'a> {
    pub pool_id:    String,
    pub period:     String,
    pub analysis:   &'a PositionAnalysis,
}

pub fn print_historical_table(inputs: &[AnalysisInput]) {
    if inputs.is_empty() {
        println!("No positions to display.");
        return;
    }

    let rows: Vec<AnalysisRow> = inputs.iter().map(build_row).collect();

    println!("\n=== Section 1 — Historical Position Analysis ===\n");
    println!("{}", Table::new(&rows));
    println!();

    print_summary(inputs);
}

fn build_row(input: &AnalysisInput) -> AnalysisRow {
    let a       = input.analysis;
    let verdict = colorize_verdict(&a.verdict.label);

    AnalysisRow {
        pool:       truncate(&input.pool_id, 12),
        period:     input.period.clone(),
        fees_usd:   format!("${:.2}", a.fees.fees_usd),
        lvr_usd:    format!("${:.2}", a.lvr.total_lvr_usd),
        ratio:      format!("{:.3}", a.verdict.ratio),
        efficiency: format!("{:.1}%", a.range_efficiency * 100.0),
        regime:     a.regime.regime.to_string(),
        verdict,
    }
}

fn colorize_verdict(label: &VerdictLabel) -> String {
    match label {
        VerdictLabel::Profitable   => label.to_string().green().to_string(),
        VerdictLabel::Marginal     => label.to_string().yellow().to_string(),
        VerdictLabel::Unprofitable => label.to_string().red().to_string(),
        VerdictLabel::Inactive     => label.to_string().dimmed().to_string(),
    }
}

fn print_summary(inputs: &[AnalysisInput]) {
    let total_fees: f64 = inputs.iter().map(|i| i.analysis.fees.fees_usd).sum();
    let total_lvr:  f64 = inputs.iter().map(|i| i.analysis.lvr.total_lvr_usd).sum();

    let weighted_ratio = if total_lvr > 0.0 {
        total_fees / total_lvr
    } else {
        0.0
    };

    let net_pnl: f64 = inputs.iter().map(|i| i.analysis.net_pnl_usd()).sum();

    let profitable_count = inputs
        .iter()
        .filter(|i| i.analysis.is_profitable())
        .count();

    println!("─── Summary ───────────────────────────────────────");
    println!("  Positions analyzed:    {}", inputs.len());
    println!("  Profitable positions:  {}/{}", profitable_count, inputs.len());
    println!("  Total fees earned:     ${:.2}", total_fees);
    println!("  Total LVR paid:        ${:.2}", total_lvr);
    println!("  Net PnL:               ${:.2}", net_pnl);
    println!(
        "  Weighted avg ratio:    {:.3}  ({})",
        weighted_ratio,
        classify_summary_ratio(weighted_ratio)
    );
    println!("───────────────────────────────────────────────────");
}

fn classify_summary_ratio(ratio: f64) -> String {
    if ratio > 1.2 {
        "overall profitable".green().to_string()
    } else if ratio >= 0.9 {
        "marginal".yellow().to_string()
    } else {
        "overall unprofitable".red().to_string()
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    format!("{}..{}", &s[..6], &s[s.len() - 4..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::accumulator::LvrResult;
    use crate::engine::fees::FeeResult;
    use crate::engine::regime::{Regime, RegimeResult};
    use crate::engine::verdict::{Verdict, VerdictLabel};

    fn make_analysis(
        fees_usd:    f64,
        lvr_usd:     f64,
        ratio:       f64,
        label:       VerdictLabel,
    ) -> PositionAnalysis {
        PositionAnalysis {
            lvr: LvrResult {
                total_lvr_usd:      lvr_usd,
                event_count:        10,
                largest_single_lvr: lvr_usd / 10.0,
                lvr_by_day:         vec![],
            },
            fees: FeeResult {
                fees_token_0: 0,
                fees_token_1: 0,
                fees_usd,
            },
            verdict: Verdict {
                ratio,
                label,
                net_pnl_usd: fees_usd - lvr_usd,
            },
            range_efficiency: 0.75,
            regime: RegimeResult {
                annualized_volatility: 0.8,
                regime:                Regime::MeanReverting,
                trending_fraction:     0.3,
            },
        }
    }

    #[test]
    fn truncate_long_address() {
        let addr = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgHkv";
        let t    = truncate(addr, 12);
        assert!(t.len() <= 14); // 6 + ".." + 4 + some margin
        assert!(t.contains(".."));
    }

    #[test]
    fn truncate_short_address_unchanged() {
        let addr = "short";
        assert_eq!(truncate(addr, 12), "short");
    }

    #[test]
    fn build_row_profitable() {
        let analysis = make_analysis(150.0, 100.0, 1.5, VerdictLabel::Profitable);
        let input    = AnalysisInput {
            pool_id:  "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgHkv".to_string(),
            period:   "90d".to_string(),
            analysis: &analysis,
        };
        let row = build_row(&input);
        assert!(row.fees_usd.contains("150.00"));
        assert!(row.lvr_usd.contains("100.00"));
        assert!(row.ratio.contains("1.500"));
        assert!(row.efficiency.contains("75.0%"));
    }

    #[test]
    fn summary_ratio_labels() {
        assert!(classify_summary_ratio(1.5).contains("profitable"));
        assert!(classify_summary_ratio(1.0).contains("marginal"));
        assert!(classify_summary_ratio(0.5).contains("unprofitable"));
    }

    #[test]
    fn empty_inputs_does_not_panic() {
        print_historical_table(&[]);
    }
}