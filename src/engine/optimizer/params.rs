#[derive(Debug, Clone, PartialEq)]
pub struct FeeTier {
    pub basis_points: u16,
}

impl FeeTier {
    pub const ONE_BPS:     FeeTier = FeeTier { basis_points: 1   };
    pub const FIVE_BPS:    FeeTier = FeeTier { basis_points: 5   };
    pub const TWENTY_FIVE: FeeTier = FeeTier { basis_points: 25  };
    pub const ONE_HUNDRED: FeeTier = FeeTier { basis_points: 100 };

    pub fn all() -> Vec<FeeTier> {
        vec![
            Self::ONE_BPS,
            Self::FIVE_BPS,
            Self::TWENTY_FIVE,
            Self::ONE_HUNDRED,
        ]
    }

    pub fn as_decimal(&self) -> f64 {
        self.basis_points as f64 / 10_000.0
    }
}

impl std::fmt::Display for FeeTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}%", self.as_decimal() * 100.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RangeWidth {
    pub percent: f64,
}

impl RangeWidth {
    pub const MIN_PERCENT:  f64 = 1.0;
    pub const MAX_PERCENT:  f64 = 25.0;
    pub const DEFAULT_STEP: f64 = 1.0;

    pub fn new(percent: f64) -> Self {
        Self { percent }
    }

    /// Generate all range widths from MIN to MAX in configurable steps
    pub fn all_with_step(step: f64) -> Vec<RangeWidth> {
        let mut widths = Vec::new();
        let mut pct    = Self::MIN_PERCENT;

        while pct <= Self::MAX_PERCENT + 1e-9 {
            widths.push(RangeWidth::new(pct));
            pct += step;
        }

        widths
    }

    pub fn all() -> Vec<RangeWidth> {
        Self::all_with_step(Self::DEFAULT_STEP)
    }

    pub fn as_decimal(&self) -> f64 {
        self.percent / 100.0
    }
}

impl std::fmt::Display for RangeWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "±{:.1}%", self.percent)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParameterSet {
    pub fee_tier:   FeeTier,
    pub range_width: RangeWidth,
}

impl ParameterSet {
    pub fn new(fee_tier: FeeTier, range_width: RangeWidth) -> Self {
        Self { fee_tier, range_width }
    }

    /// Generate all combinations of fee tiers and range widths
    pub fn all() -> Vec<ParameterSet> {
        let tiers  = FeeTier::all();
        let widths = RangeWidth::all();

        tiers
            .into_iter()
            .flat_map(|tier| {
                widths
                    .iter()
                    .cloned()
                    .map(move |width| ParameterSet::new(tier.clone(), width))
            })
            .collect()
    }

    /// Generate search space with custom step size
    pub fn all_with_step(step: f64) -> Vec<ParameterSet> {
        let tiers  = FeeTier::all();
        let widths = RangeWidth::all_with_step(step);

        tiers
            .into_iter()
            .flat_map(|tier| {
                widths
                    .iter()
                    .cloned()
                    .map(move |width| ParameterSet::new(tier.clone(), width))
            })
            .collect()
    }
}

impl std::fmt::Display for ParameterSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} range {}", self.fee_tier, self.range_width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_tiers_have_correct_basis_points() {
        assert_eq!(FeeTier::ONE_BPS.basis_points,     1);
        assert_eq!(FeeTier::FIVE_BPS.basis_points,    5);
        assert_eq!(FeeTier::TWENTY_FIVE.basis_points, 25);
        assert_eq!(FeeTier::ONE_HUNDRED.basis_points, 100);
    }

    #[test]
    fn fee_tier_decimal_conversion() {
        assert!((FeeTier::TWENTY_FIVE.as_decimal() - 0.0025).abs() < 1e-10);
        assert!((FeeTier::ONE_HUNDRED.as_decimal() - 0.01).abs()   < 1e-10);
    }

    #[test]
    fn range_width_all_has_25_entries() {
        // 1.0% to 25.0% in 1.0% steps = 25 entries
        let widths = RangeWidth::all();
        assert_eq!(widths.len(), 25);
        assert!((widths.first().unwrap().percent - 1.0).abs()  < 1e-9);
        assert!((widths.last().unwrap().percent  - 25.0).abs() < 1e-9);
    }

    #[test]
    fn parameter_set_all_has_correct_count() {
        // 4 fee tiers × 25 range widths = 100 combinations
        let params = ParameterSet::all();
        assert_eq!(params.len(), 100);
    }

    #[test]
    fn parameter_set_all_with_step_2() {
        // 4 tiers × 13 widths (1,3,5,...,25) = 52 combinations
        let params = ParameterSet::all_with_step(2.0);
        assert_eq!(params.len(), 52);
    }

    #[test]
    fn all_combinations_are_unique() {
        let params = ParameterSet::all();
        let mut seen = std::collections::HashSet::new();
        for p in &params {
            let key = (p.fee_tier.basis_points, (p.range_width.percent * 100.0) as u32);
            assert!(seen.insert(key), "Duplicate parameter set: {}", p);
        }
    }

    #[test]
    fn display_formatting() {
        let p = ParameterSet::new(FeeTier::TWENTY_FIVE, RangeWidth::new(8.0));
        assert_eq!(p.to_string(), "0.25% range ±8.0%");
    }

    #[test]
    fn four_fee_tiers_defined() {
        assert_eq!(FeeTier::all().len(), 4);
    }
}