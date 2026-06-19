use anyhow::{bail, Context, Result};
use chrono::{Local, NaiveDate};

const MAX_RANGE_DAYS: i64 = 90;
const DATE_FORMAT: &str = "%Y-%m-%d";
#[derive(Clone, Debug)]
pub struct DateRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

impl DateRange {
    pub fn parse(from: &str, to: &str) -> Result<Self> {
        let from = NaiveDate::parse_from_str(from, DATE_FORMAT)
            .with_context(|| format!("Invalid --from date: {:?}", from))?;

        let to = NaiveDate::parse_from_str(to, DATE_FORMAT)
            .with_context(|| format!("Invalid --to date: {:?}", to))?;

        let today = Local::now().date_naive();

        if to > today {
            bail!("--to date {} is in the future (today is {})", to, today);
        }

        if from >= to {
            bail!("--from date {} must be before --to date {}", from, to);
        }

        let days = (to - from).num_days();
        if days > MAX_RANGE_DAYS {
            bail!(
                "Date range of {} days exceeds the {} day maximum",
                days,
                MAX_RANGE_DAYS
            );
        }

        Ok(Self { from, to })
    }

    pub fn from_date(&self) -> NaiveDate {
        self.from
    }

    pub fn to_date(&self) -> NaiveDate {
        self.to
    }

    pub fn num_days(&self) -> i64 {
        (self.to - self.from).num_days()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_range_passes() {
        let result = DateRange::parse("2025-01-01", "2025-03-31");
        assert!(result.is_ok());
        let dr = result.unwrap();
        assert_eq!(dr.num_days(), 89);
    }

    #[test]
    fn reversed_range_fails() {
        let result = DateRange::parse("2025-03-31", "2025-01-01");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("before"));
    }

    #[test]
    fn future_end_date_fails() {
        let result = DateRange::parse("2025-01-01", "2099-12-31");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("future"));
    }

    #[test]
    fn range_exceeding_90_days_fails() {
        // 2025-01-01 to 2025-04-15 = 104 days
        let result = DateRange::parse("2025-01-01", "2025-04-15");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("exceeds"));
    }
}