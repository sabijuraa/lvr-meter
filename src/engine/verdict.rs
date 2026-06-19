use crate::engine::accumulator::LvrResult;
use crate::engine::fees::FeeResult;

#[derive(Debug, Clone, PartialEq)]
pub enum VerdictLabel {
    Profitable,
    Marginal,
    Unprofitable,
    Inactive,
}

impl std::fmt::Display for VerdictLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerdictLabel::Profitable   => write!(f, "PROFITABLE"),
            VerdictLabel::Marginal     => write!(f, "MARGINAL"),
            VerdictLabel::Unprofitable => write!(f, "UNPROFITABLE"),
            VerdictLabel::Inactive     => write!(f, "INACTIVE"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Verdict {
    pub ratio:       f64,
    pub label:       VerdictLabel,
    pub net_pnl_usd: f64,
}

impl Verdict {
    pub fn is_profitable(&self) -> bool {
        matches!(self.label, VerdictLabel::Profitable)
    }
}

const PROFITABLE_THRESHOLD:   f64 = 1.2;
const MARGINAL_LOWER:         f64 = 0.9;
const INACTIVE_EVENT_MINIMUM: usize = 5;

pub fn compute_verdict(lvr: &LvrResult, fees: &FeeResult) -> Verdict {
    let net_pnl_usd = fees.fees_usd - lvr.total_lvr_usd;

    // Not enough swap events to classify meaningfully
    if lvr.event_count < INACTIVE_EVENT_MINIMUM {
        return Verdict {
            ratio: 0.0,
            label: VerdictLabel::Inactive,
            net_pnl_usd,
        };
    }

    if lvr.total_lvr_usd == 0.0 {
        return Verdict {
            ratio: f64::INFINITY,
            label: VerdictLabel::Profitable,
            net_pnl_usd,
        };
    }

    let ratio = fees.fees_usd / lvr.total_lvr_usd;

    let label = classify(ratio);

    Verdict { ratio, label, net_pnl_usd }
}

fn classify(ratio: f64) -> VerdictLabel {
    if ratio > PROFITABLE_THRESHOLD {
        VerdictLabel::Profitable
    } else if ratio >= MARGINAL_LOWER {
        VerdictLabel::Marginal
    } else {
        VerdictLabel::Unprofitable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_lvr(total: f64, count: usize) -> LvrResult {
        LvrResult {
            total_lvr_usd:      total,
            event_count:        count,
            largest_single_lvr: total / count.max(1) as f64,
            lvr_by_day:         vec![],
        }
    }

    fn make_fees(usd: f64) -> FeeResult {
        FeeResult {
            fees_token_0: 0,
            fees_token_1: 0,
            fees_usd:     usd,
        }
    }

    #[test]
    fn profitable_above_1_2() {
        let lvr     = make_lvr(100.0, 10);
        let fees    = make_fees(125.0); // ratio = 1.25
        let verdict = compute_verdict(&lvr, &fees);
        assert_eq!(verdict.label, VerdictLabel::Profitable);
        assert!((verdict.ratio - 1.25).abs() < 1e-9);
    }

    #[test]
    fn marginal_at_exactly_1_2() {
        let lvr     = make_lvr(100.0, 10);
        let fees    = make_fees(120.0); // ratio = 1.2 exactly
        let verdict = compute_verdict(&lvr, &fees);
        assert_eq!(verdict.label, VerdictLabel::Marginal);
    }

    #[test]
    fn marginal_between_0_9_and_1_2() {
        let lvr     = make_lvr(100.0, 10);
        let fees    = make_fees(105.0); // ratio = 1.05
        let verdict = compute_verdict(&lvr, &fees);
        assert_eq!(verdict.label, VerdictLabel::Marginal);
    }

    #[test]
    fn marginal_at_exactly_0_9() {
        let lvr     = make_lvr(100.0, 10);
        let fees    = make_fees(90.0); // ratio = 0.9 exactly
        let verdict = compute_verdict(&lvr, &fees);
        assert_eq!(verdict.label, VerdictLabel::Marginal);
    }

    #[test]
    fn unprofitable_below_0_9() {
        let lvr     = make_lvr(100.0, 10);
        let fees    = make_fees(80.0); // ratio = 0.8
        let verdict = compute_verdict(&lvr, &fees);
        assert_eq!(verdict.label, VerdictLabel::Unprofitable);
    }

    #[test]
    fn unprofitable_at_zero_fees() {
        let lvr     = make_lvr(100.0, 10);
        let fees    = make_fees(0.0); // ratio = 0.0
        let verdict = compute_verdict(&lvr, &fees);
        assert_eq!(verdict.label, VerdictLabel::Unprofitable);
    }

    #[test]
    fn inactive_when_too_few_events() {
        let lvr     = make_lvr(100.0, 4); // below INACTIVE_EVENT_MINIMUM
        let fees    = make_fees(200.0);
        let verdict = compute_verdict(&lvr, &fees);
        assert_eq!(verdict.label, VerdictLabel::Inactive);
        assert_eq!(verdict.ratio, 0.0);
    }

    #[test]
    fn inactive_at_exactly_minimum_minus_one() {
        let lvr     = make_lvr(100.0, INACTIVE_EVENT_MINIMUM - 1);
        let fees    = make_fees(200.0);
        let verdict = compute_verdict(&lvr, &fees);
        assert_eq!(verdict.label, VerdictLabel::Inactive);
    }

    #[test]
    fn active_at_exactly_minimum() {
        let lvr     = make_lvr(100.0, INACTIVE_EVENT_MINIMUM);
        let fees    = make_fees(200.0); // ratio = 2.0 → Profitable
        let verdict = compute_verdict(&lvr, &fees);
        assert_ne!(verdict.label, VerdictLabel::Inactive);
    }

    #[test]
    fn net_pnl_is_fees_minus_lvr() {
        let lvr     = make_lvr(100.0, 10);
        let fees    = make_fees(150.0);
        let verdict = compute_verdict(&lvr, &fees);
        assert!((verdict.net_pnl_usd - 50.0).abs() < 1e-9);
    }

    #[test]
    fn negative_net_pnl_when_lvr_exceeds_fees() {
        let lvr     = make_lvr(200.0, 10);
        let fees    = make_fees(100.0);
        let verdict = compute_verdict(&lvr, &fees);
        assert!(verdict.net_pnl_usd < 0.0);
        assert!((verdict.net_pnl_usd - (-100.0)).abs() < 1e-9);
    }

    #[test]
    fn zero_lvr_with_fees_is_profitable() {
        let lvr     = make_lvr(0.0, 10);
        let fees    = make_fees(100.0);
        let verdict = compute_verdict(&lvr, &fees);
        assert_eq!(verdict.label, VerdictLabel::Profitable);
        assert_eq!(verdict.ratio, f64::INFINITY);
    }

    #[test]
    fn display_labels_are_uppercase() {
        assert_eq!(VerdictLabel::Profitable.to_string(),   "PROFITABLE");
        assert_eq!(VerdictLabel::Marginal.to_string(),     "MARGINAL");
        assert_eq!(VerdictLabel::Unprofitable.to_string(), "UNPROFITABLE");
        assert_eq!(VerdictLabel::Inactive.to_string(),     "INACTIVE");
    }
}