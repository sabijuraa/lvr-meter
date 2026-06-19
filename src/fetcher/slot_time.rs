use anyhow::{bail, Result};

use crate::fetcher::rpc::RpcClientWrapper;

/// Approximate slots per second on Solana mainnet
const SLOTS_PER_SECOND: u64 = 2;

/// Maximum binary search iterations before giving up
const MAX_ITERATIONS: u32 = 50;

/// Acceptable slot precision — within 100 slots (~40 seconds)
const SLOT_TOLERANCE: i64 = 100;

/// Estimate the slot number closest to a given Unix timestamp.
/// Uses binary search with getBlockTime RPC calls.
pub fn estimate_slot_for_timestamp(
    unix_ts: i64,
    client: &RpcClientWrapper,
) -> Result<u64> {
    let current_slot = client
        .call_with_retry("get_slot", || client.client.get_slot())?;

    let current_ts = client
        .call_with_retry("get_block_time_current", || {
            client.client.get_block_time(current_slot)
        })?;

    if unix_ts >= current_ts {
        bail!(
            "Target timestamp {} is in the future (current: {})",
            unix_ts,
            current_ts
        );
    }

    // Initial bracket: estimate low slot using known slot rate
    let seconds_ago = (current_ts - unix_ts).max(0) as u64;
    let estimated_low = current_slot.saturating_sub(seconds_ago * SLOTS_PER_SECOND * 2);

    let mut low  = estimated_low;
    let mut high = current_slot;

    for _ in 0..MAX_ITERATIONS {
        if high <= low + 1 {
            break;
        }

        let mid = low + (high - low) / 2;

        let mid_ts = match client.call_with_retry("get_block_time_mid", || {
            client.client.get_block_time(mid)
        }) {
            Ok(ts)  => ts,
            Err(_)  => {
                // Slot was skipped — nudge mid and continue
                high = mid;
                continue;
            }
        };

        let diff = (mid_ts - unix_ts).abs();
        if diff <= SLOT_TOLERANCE {
            return Ok(mid);
        }

        if mid_ts < unix_ts {
            low = mid;
        } else {
            high = mid;
        }
    }

    Ok(low + (high - low) / 2)
}

/// Convert a NaiveDate to a Unix timestamp at midnight UTC
pub fn date_to_unix_ts(date: chrono::NaiveDate) -> i64 {
    date.and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn date_to_unix_ts_known_value() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let ts   = date_to_unix_ts(date);
        // 2025-01-01 00:00:00 UTC = 1735689600
        assert_eq!(ts, 1_735_689_600);
    }

    #[test]
    fn date_to_unix_ts_another_known_value() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        let ts   = date_to_unix_ts(date);
        // 2024-03-15 00:00:00 UTC = 1710460800
        assert_eq!(ts, 1_710_460_800);
    }

    #[test]
    fn slot_tolerance_constant_is_reasonable() {
        // 100 slots at ~0.4s each = ~40 seconds of imprecision
        // For daily date ranges this is negligible
        assert!(SLOT_TOLERANCE <= 200);
    }
}