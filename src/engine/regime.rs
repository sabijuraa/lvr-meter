use crate::parser::types::SwapEvent;

#[derive(Debug, Clone, PartialEq)]
pub enum Regime {
    Trending,
    MeanReverting,
    Volatile,
}

impl std::fmt::Display for Regime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Regime::Trending      => write!(f, "Trending"),
            Regime::MeanReverting => write!(f, "Mean-Reverting"),
            Regime::Volatile      => write!(f, "Volatile"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegimeResult {
    pub annualized_volatility: f64,
    pub regime:                Regime,
    pub trending_fraction:     f64,
}

const TRENDING_THRESHOLD:    f64 = 0.7;
const VOLATILE_THRESHOLD:    f64 = 0.3;
const VOLATILE_VOL_FLOOR:    f64 = 0.6; // 60% annualized
const SECONDS_PER_YEAR:      f64 = 365.25 * 24.0 * 3600.0;
const MIN_EVENTS_FOR_REGIME: usize = 3;

pub fn classify_regime(events: &[SwapEvent]) -> RegimeResult {
    if events.len() < MIN_EVENTS_FOR_REGIME {
        return RegimeResult {
            annualized_volatility: 0.0,
            regime:                Regime::MeanReverting,
            trending_fraction:     0.0,
        };
    }

    let log_returns = compute_log_returns(events);
    let annualized_volatility = annualize_volatility(&log_returns, events);
    let trending_fraction     = compute_trending_fraction(events);
    let regime                = classify(trending_fraction, annualized_volatility);

    RegimeResult { annualized_volatility, regime, trending_fraction }
}

/// Compute log-returns: ln(price_after / price_before) for each event
fn compute_log_returns(events: &[SwapEvent]) -> Vec<f64> {
    events
        .iter()
        .filter(|e| e.price_before > 0.0 && e.price_after > 0.0)
        .map(|e| (e.price_after / e.price_before).ln())
        .collect()
}

/// Standard deviation of a slice of f64
fn std_dev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|v| (v - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;

    variance.sqrt()
}

/// Annualize per-event volatility by the observed event frequency
fn annualize_volatility(log_returns: &[f64], events: &[SwapEvent]) -> f64 {
    if log_returns.len() < 2 {
        return 0.0;
    }

    let per_event_vol = std_dev(log_returns);

    // Estimate events per year from observed time span
    let events_per_year = estimate_events_per_year(events);

    per_event_vol * events_per_year.sqrt()
}

/// Estimate how many events occur per year based on observed frequency
fn estimate_events_per_year(events: &[SwapEvent]) -> f64 {
    let first = events.first().unwrap();
    let last  = events.last().unwrap();

    let time_span_secs = (last.timestamp - first.timestamp).abs() as f64;

    if time_span_secs < 1.0 {
        // Fall back to slot-based estimate if timestamps are missing
        let slot_span = last.slot.saturating_sub(first.slot) as f64;
        if slot_span < 1.0 {
            return events.len() as f64; // assume 1-year window
        }
        // ~2.5 slots/sec on Solana
        let estimated_secs = slot_span / 2.5;
        let freq = events.len() as f64 / estimated_secs;
        return freq * SECONDS_PER_YEAR;
    }

    let freq = events.len() as f64 / time_span_secs;
    freq * SECONDS_PER_YEAR
}

/// Trending fraction: net price movement / total absolute movement
///
/// A value near 1.0 means all moves were in one direction (trending).
/// A value near 0.0 means moves cancelled out (mean-reverting).
fn compute_trending_fraction(events: &[SwapEvent]) -> f64 {
    let total_abs: f64 = events
        .iter()
        .map(|e| (e.price_after - e.price_before).abs())
        .sum();

    if total_abs < 1e-12 {
        return 0.0;
    }

    let first_price = events.first().unwrap().price_before;
    let last_price  = events.last().unwrap().price_after;
    let net_move    = (last_price - first_price).abs();

    (net_move / total_abs).min(1.0)
}

fn classify(trending_fraction: f64, annualized_volatility: f64) -> Regime {
    if trending_fraction > TRENDING_THRESHOLD {
        Regime::Trending
    } else if trending_fraction < VOLATILE_THRESHOLD
        && annualized_volatility > VOLATILE_VOL_FLOOR
    {
        Regime::Volatile
    } else {
        Regime::MeanReverting
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::types::{SwapDirection, SwapEvent};
    use solana_sdk::pubkey::Pubkey;

    fn make_event(
        slot:        u64,
        timestamp:   i64,
        price_before: f64,
        price_after:  f64,
    ) -> SwapEvent {
        SwapEvent {
            slot,
            timestamp,
            pool:              Pubkey::new_unique(),
            price_before,
            price_after,
            sqrt_price_before: 0,
            sqrt_price_after:  0,
            active_liquidity:  1_000_000,
            fee_rate:          25,
            direction:         if price_after < price_before {
                SwapDirection::ZeroForOne
            } else {
                SwapDirection::OneForZero
            },
        }
    }

    fn make_events_at_times(prices: &[(f64, f64)], timestamps: &[i64]) -> Vec<SwapEvent> {
        prices
            .iter()
            .zip(timestamps.iter())
            .enumerate()
            .map(|(i, ((before, after), ts))| {
                make_event(i as u64 * 100, *ts, *before, *after)
            })
            .collect()
    }

    // Timestamps spanning ~1 day
    const BASE_TS: i64 = 1_735_689_600; // 2025-01-01
    const HOUR:    i64 = 3600;

    #[test]
    fn trending_when_price_moves_consistently_up() {
        // All moves in one direction — net/total = 1.0
        let prices: Vec<(f64, f64)> = (0..10)
            .map(|i| (100.0 + i as f64, 101.0 + i as f64))
            .collect();
        let timestamps: Vec<i64> = (0..10).map(|i| BASE_TS + i * HOUR).collect();
        let events = make_events_at_times(&prices, &timestamps);

        let result = classify_regime(&events);
        assert_eq!(result.regime, Regime::Trending);
        assert!(result.trending_fraction > TRENDING_THRESHOLD);
    }

    #[test]
    fn mean_reverting_when_price_oscillates() {
        // Alternating up/down moves — net movement near zero
        let prices: Vec<(f64, f64)> = (0..10)
            .map(|i| {
                if i % 2 == 0 {
                    (100.0, 101.0) // up
                } else {
                    (101.0, 100.0) // down
                }
            })
            .collect();
        let timestamps: Vec<i64> = (0..10).map(|i| BASE_TS + i * HOUR).collect();
        let events = make_events_at_times(&prices, &timestamps);

        let result = classify_regime(&events);
        assert_eq!(result.regime, Regime::MeanReverting);
    }

    #[test]
    fn volatile_when_large_oscillations() {
        // Large alternating moves with high per-event volatility
        // trending_fraction will be low (oscillating) and vol will be high
        let prices: Vec<(f64, f64)> = (0..20)
            .map(|i| {
                if i % 2 == 0 {
                    (100.0, 200.0) // +100%
                } else {
                    (200.0, 100.0) // -50%
                }
            })
            .collect();
        // Compress timestamps so events/year is very high → high annualized vol
        let timestamps: Vec<i64> = (0..20).map(|i| BASE_TS + i * 60).collect();
        let events = make_events_at_times(&prices, &timestamps);

        let result = classify_regime(&events);
        assert!(
            result.annualized_volatility > VOLATILE_VOL_FLOOR,
            "Expected high vol, got {}",
            result.annualized_volatility
        );
    }

    #[test]
    fn few_events_returns_mean_reverting_default() {
        let events = vec![
            make_event(0, BASE_TS,        100.0, 101.0),
            make_event(1, BASE_TS + HOUR, 101.0, 100.0),
        ];
        let result = classify_regime(&events);
        assert_eq!(result.regime, Regime::MeanReverting);
        assert_eq!(result.annualized_volatility, 0.0);
    }

    #[test]
    fn trending_fraction_is_between_zero_and_one() {
        let prices: Vec<(f64, f64)> = (0..10)
            .map(|i| (100.0 + i as f64 * 0.1, 100.1 + i as f64 * 0.1))
            .collect();
        let timestamps: Vec<i64> = (0..10).map(|i| BASE_TS + i * HOUR).collect();
        let events = make_events_at_times(&prices, &timestamps);

        let result = classify_regime(&events);
        assert!(
            result.trending_fraction >= 0.0 && result.trending_fraction <= 1.0,
            "trending_fraction {} out of [0,1]",
            result.trending_fraction
        );
    }

    #[test]
    fn annualized_volatility_is_non_negative() {
        let prices     = vec![(100.0, 101.0), (101.0, 99.0), (99.0, 100.0)];
        let timestamps = vec![BASE_TS, BASE_TS + HOUR, BASE_TS + HOUR * 2];
        let events     = make_events_at_times(&prices, &timestamps);

        let result = classify_regime(&events);
        assert!(
            result.annualized_volatility >= 0.0,
            "Volatility {} is negative",
            result.annualized_volatility
        );
    }

    #[test]
    fn display_regime_labels() {
        assert_eq!(Regime::Trending.to_string(),      "Trending");
        assert_eq!(Regime::MeanReverting.to_string(), "Mean-Reverting");
        assert_eq!(Regime::Volatile.to_string(),      "Volatile");
    }

    #[test]
    fn zero_price_moves_give_zero_trending_fraction() {
        let prices     = vec![(100.0, 100.0), (100.0, 100.0), (100.0, 100.0)];
        let timestamps = vec![BASE_TS, BASE_TS + HOUR, BASE_TS + HOUR * 2];
        let events     = make_events_at_times(&prices, &timestamps);

        let result = classify_regime(&events);
        assert_eq!(result.trending_fraction, 0.0);
    }
}