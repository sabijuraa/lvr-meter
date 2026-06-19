use chrono::{DateTime, NaiveDate};

use crate::engine::lvr::compute_instantaneous_lvr;
use crate::parser::types::SwapEvent;

#[derive(Debug, Clone)]
pub struct LvrResult {
    pub total_lvr_usd:     f64,
    pub event_count:       usize,
    pub largest_single_lvr: f64,
    pub lvr_by_day:        Vec<(NaiveDate, f64)>,
}

impl LvrResult {
    pub fn average_lvr_per_event(&self) -> f64 {
        if self.event_count == 0 {
            return 0.0;
        }
        self.total_lvr_usd / self.event_count as f64
    }
}

pub fn compute_total_lvr(events: &[SwapEvent]) -> LvrResult {
    if events.is_empty() {
        return LvrResult {
            total_lvr_usd:      0.0,
            event_count:        0,
            largest_single_lvr: 0.0,
            lvr_by_day:         vec![],
        };
    }

    let mut total_lvr_usd      = 0.0f64;
    let mut largest_single_lvr = 0.0f64;
    let mut daily: std::collections::BTreeMap<NaiveDate, f64> =
        std::collections::BTreeMap::new();

    for event in events {
        let lvr = compute_instantaneous_lvr(event);

        total_lvr_usd      += lvr;
        largest_single_lvr  = largest_single_lvr.max(lvr);

        let date = timestamp_to_date(event.timestamp);
        *daily.entry(date).or_insert(0.0) += lvr;
    }

    let lvr_by_day: Vec<(NaiveDate, f64)> = daily.into_iter().collect();

    LvrResult {
        total_lvr_usd,
        event_count: events.len(),
        largest_single_lvr,
        lvr_by_day,
    }
}

fn timestamp_to_date(unix_ts: i64) -> NaiveDate {
    DateTime::from_timestamp(unix_ts, 0)
        .unwrap_or_else(|| DateTime::from_timestamp(0, 0).unwrap())
        .date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::types::{SwapDirection, SwapEvent};
    use solana_sdk::pubkey::Pubkey;

    fn make_event(
        price_before:  f64,
        price_after:   f64,
        liquidity_usd: f64,
        timestamp:     i64,
    ) -> SwapEvent {
        let raw_liquidity = (liquidity_usd / price_before * 1e18) as u128;
        SwapEvent {
            slot:              1,
            timestamp,
            pool:              Pubkey::new_unique(),
            price_before,
            price_after,
            sqrt_price_before: 0,
            sqrt_price_after:  0,
            active_liquidity:  raw_liquidity,
            fee_rate:          25,
            direction:         SwapDirection::ZeroForOne,
        }
    }

    // 2025-01-01 00:00:00 UTC
    const DAY1: i64 = 1_735_689_600;
    // 2025-01-02 00:00:00 UTC
    const DAY2: i64 = 1_735_776_000;

    #[test]
    fn total_matches_sum_of_individual_lvrs() {
        let events: Vec<SwapEvent> = (0..10)
            .map(|_| make_event(100.0, 100.5, 1_000_000.0, DAY1))
            .collect();

        let individual_sum: f64 = events
            .iter()
            .map(|e| compute_instantaneous_lvr(e))
            .sum();

        let result = compute_total_lvr(&events);

        let diff = (result.total_lvr_usd - individual_sum).abs();
        assert!(
            diff < 1e-6,
            "Total {} does not match sum {} (diff {})",
            result.total_lvr_usd, individual_sum, diff
        );
    }

    #[test]
    fn event_count_matches_input_length() {
        let events: Vec<SwapEvent> = (0..10)
            .map(|_| make_event(100.0, 100.5, 1_000_000.0, DAY1))
            .collect();

        let result = compute_total_lvr(&events);
        assert_eq!(result.event_count, 10);
    }

    #[test]
    fn largest_single_lvr_is_correct() {
        let mut events = vec![
            make_event(100.0, 100.5, 1_000_000.0, DAY1), // small move
            make_event(100.0, 102.0, 1_000_000.0, DAY1), // large move
            make_event(100.0, 100.1, 1_000_000.0, DAY1), // tiny move
        ];

        let lvrs: Vec<f64> = events.iter().map(compute_instantaneous_lvr).collect();
        let expected_max   = lvrs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let result = compute_total_lvr(&events);
        let diff   = (result.largest_single_lvr - expected_max).abs();

        assert!(
            diff < 1e-6,
            "Largest LVR {} does not match expected {}",
            result.largest_single_lvr, expected_max
        );
    }

    #[test]
    fn daily_breakdown_groups_by_date() {
        let events = vec![
            make_event(100.0, 100.5, 1_000_000.0, DAY1),
            make_event(100.0, 100.5, 1_000_000.0, DAY1),
            make_event(100.0, 100.5, 1_000_000.0, DAY2),
        ];

        let result = compute_total_lvr(&events);

        assert_eq!(result.lvr_by_day.len(), 2, "Should have 2 days");

        let day1_total = result.lvr_by_day
            .iter()
            .find(|(d, _)| *d == NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
            .map(|(_, v)| *v)
            .unwrap_or(0.0);

        let day2_total = result.lvr_by_day
            .iter()
            .find(|(d, _)| *d == NaiveDate::from_ymd_opt(2025, 1, 2).unwrap())
            .map(|(_, v)| *v)
            .unwrap_or(0.0);

        // Day 1 has 2 events, Day 2 has 1 — so Day 1 total should be ~2x Day 2
        let ratio = day1_total / day2_total;
        assert!(
            (ratio - 2.0).abs() < 1e-6,
            "Day 1 total should be 2x Day 2: {} vs {}",
            day1_total, day2_total
        );
    }

    #[test]
    fn daily_breakdown_is_sorted_ascending() {
        let events = vec![
            make_event(100.0, 100.5, 1_000_000.0, DAY2),
            make_event(100.0, 100.5, 1_000_000.0, DAY1),
        ];

        let result = compute_total_lvr(&events);

        for i in 1..result.lvr_by_day.len() {
            assert!(
                result.lvr_by_day[i].0 >= result.lvr_by_day[i - 1].0,
                "Daily breakdown not sorted"
            );
        }
    }

    #[test]
    fn empty_events_returns_zero_result() {
        let result = compute_total_lvr(&[]);
        assert_eq!(result.total_lvr_usd,      0.0);
        assert_eq!(result.event_count,         0);
        assert_eq!(result.largest_single_lvr,  0.0);
        assert!(result.lvr_by_day.is_empty());
    }

    #[test]
    fn average_lvr_per_event_correct() {
        let events: Vec<SwapEvent> = (0..5)
            .map(|_| make_event(100.0, 100.5, 1_000_000.0, DAY1))
            .collect();

        let result   = compute_total_lvr(&events);
        let expected = result.total_lvr_usd / 5.0;
        let diff     = (result.average_lvr_per_event() - expected).abs();

        assert!(diff < 1e-9);
    }
}