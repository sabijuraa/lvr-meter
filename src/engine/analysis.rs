use anyhow::Result;

use crate::engine::accumulator::{compute_total_lvr, LvrResult};
use crate::engine::fees::{compute_fees_earned, FeeResult};
use crate::engine::range_efficiency::compute_range_efficiency;
use crate::engine::regime::{classify_regime, RegimeResult};
use crate::engine::verdict::{compute_verdict, Verdict};
use crate::fetcher::raydium::pool_state::PoolState;
use crate::fetcher::raydium::types::PersonalPositionState;
use crate::parser::types::SwapEvent;

#[derive(Debug, Clone)]
pub struct PositionAnalysis {
    pub lvr:              LvrResult,
    pub fees:             FeeResult,
    pub verdict:          Verdict,
    pub range_efficiency: f64,
    pub regime:           RegimeResult,
}

impl PositionAnalysis {
    pub fn compute(
        events:     &[SwapEvent],
        position:   &PersonalPositionState,
        pool_open:  &PoolState,
        pool_close: &PoolState,
    ) -> Result<Self> {
        let lvr  = compute_total_lvr(events);
        let fees = compute_fees_earned(position, pool_open, pool_close);

        let range_efficiency = compute_range_efficiency(
            events,
            position.tick_lower_index,
            position.tick_upper_index,
        );

        let regime  = classify_regime(events);
        let verdict = compute_verdict(&lvr, &fees);

        tracing::info!(
            "Position analysis: LVR=${:.2} fees=${:.2} ratio={:.3} efficiency={:.2} regime={}",
            lvr.total_lvr_usd,
            fees.fees_usd,
            verdict.ratio,
            range_efficiency,
            regime.regime,
        );

        Ok(Self { lvr, fees, verdict, range_efficiency, regime })
    }

    pub fn fee_to_lvr_ratio(&self) -> f64 {
        self.verdict.ratio
    }

    pub fn net_pnl_usd(&self) -> f64 {
        self.verdict.net_pnl_usd
    }

    pub fn is_profitable(&self) -> bool {
        self.verdict.is_profitable()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "LVR=${:.2} | Fees=${:.2} | Ratio={:.3} | Efficiency={:.1}% | {} | {}",
            self.lvr.total_lvr_usd,
            self.fees.fees_usd,
            self.verdict.ratio,
            self.range_efficiency * 100.0,
            self.regime.regime,
            self.verdict.label,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetcher::raydium::pool_state::POOL_STATE_DISCRIMINATOR;
    use crate::fetcher::raydium::types::REWARD_NUM;
    use crate::parser::types::{SwapDirection, SwapEvent};
    use solana_sdk::pubkey::Pubkey;

    fn make_pool_state(sqrt_price: u128, fee_growth: u128) -> PoolState {
        let mut data = vec![0u8; 512];
        data[..8].copy_from_slice(&POOL_STATE_DISCRIMINATOR);
        data[8 + 225] = 9;
        data[8 + 226] = 6;
        data[8 + 227..8 + 229].copy_from_slice(&1u16.to_le_bytes());
        data[8 + 229..8 + 245].copy_from_slice(&1_000_000u128.to_le_bytes());
        data[8 + 245..8 + 261].copy_from_slice(&sqrt_price.to_le_bytes());
        data[8 + 261..8 + 265].copy_from_slice(&0i32.to_le_bytes());
        data[8 + 269..8 + 285].copy_from_slice(&fee_growth.to_le_bytes());
        data[8 + 285..8 + 301].copy_from_slice(&fee_growth.to_le_bytes());
        PoolState::from_account_data(&data).unwrap()
    }

    fn make_position(tick_lower: i32, tick_upper: i32) -> PersonalPositionState {
        PersonalPositionState {
            bump:                         [255],
            nft_mint:                     Pubkey::new_unique(),
            pool_id:                      Pubkey::new_unique(),
            tick_lower_index:             tick_lower,
            tick_upper_index:             tick_upper,
            liquidity:                    1_000_000,
            fee_growth_inside_0_last_x64: 0,
            fee_growth_inside_1_last_x64: 0,
            token_fees_owed_0:            0,
            token_fees_owed_1:            0,
            reward_infos:                 Default::default(),
            recent_epoch:                 0,
            padding:                      [0; 7],
        }
    }

    fn make_event(slot: u64, price_before: f64, price_after: f64) -> SwapEvent {
        let raw_liquidity = (1_000_000.0_f64 / price_before * 1e18) as u128;
        SwapEvent {
            slot,
            timestamp:         1_735_689_600 + slot as i64 * 400,
            pool:              Pubkey::new_unique(),
            price_before,
            price_after,
            sqrt_price_before: 0,
            sqrt_price_after:  0,
            active_liquidity:  raw_liquidity,
            fee_rate:          25,
            direction:         if price_after < price_before {
                SwapDirection::ZeroForOne
            } else {
                SwapDirection::OneForZero
            },
        }
    }

    #[test]
    fn compute_returns_ok_for_valid_inputs() {
        let sqrt = crate::parser::price::price_to_sqrt_price_x64(150.0, 9, 6);
        let pool = make_pool_state(sqrt, 0);
        let pos  = make_position(-1000, 1000);

        let events: Vec<SwapEvent> = (0..10)
            .map(|i| make_event(i * 100, 150.0, 150.5))
            .collect();

        let result = PositionAnalysis::compute(&events, &pos, &pool, &pool);
        assert!(result.is_ok());
    }

    #[test]
    fn range_efficiency_between_zero_and_one() {
        let sqrt = crate::parser::price::price_to_sqrt_price_x64(150.0, 9, 6);
        let pool = make_pool_state(sqrt, 0);
        let pos  = make_position(-1000, 1000);

        let events: Vec<SwapEvent> = (0..10)
            .map(|i| make_event(i * 100, 150.0, 150.5))
            .collect();

        let analysis = PositionAnalysis::compute(&events, &pos, &pool, &pool).unwrap();
        assert!(
            analysis.range_efficiency >= 0.0 && analysis.range_efficiency <= 1.0,
            "range_efficiency {} out of bounds",
            analysis.range_efficiency
        );
    }

    #[test]
    fn net_pnl_equals_fees_minus_lvr() {
        let sqrt = crate::parser::price::price_to_sqrt_price_x64(150.0, 9, 6);
        let pool = make_pool_state(sqrt, 0);
        let pos  = make_position(-1000, 1000);

        let events: Vec<SwapEvent> = (0..10)
            .map(|i| make_event(i * 100, 150.0, 150.5))
            .collect();

        let analysis = PositionAnalysis::compute(&events, &pos, &pool, &pool).unwrap();
        let expected = analysis.fees.fees_usd - analysis.lvr.total_lvr_usd;
        let diff     = (analysis.net_pnl_usd() - expected).abs();
        assert!(diff < 1e-9);
    }

    #[test]
    fn summary_line_contains_key_fields() {
        let sqrt = crate::parser::price::price_to_sqrt_price_x64(150.0, 9, 6);
        let pool = make_pool_state(sqrt, 0);
        let pos  = make_position(-1000, 1000);

        let events: Vec<SwapEvent> = (0..10)
            .map(|i| make_event(i * 100, 150.0, 150.5))
            .collect();

        let analysis = PositionAnalysis::compute(&events, &pos, &pool, &pool).unwrap();
        let summary  = analysis.summary_line();

        assert!(summary.contains("LVR="));
        assert!(summary.contains("Fees="));
        assert!(summary.contains("Ratio="));
        assert!(summary.contains("Efficiency="));
    }

    #[test]
    fn empty_events_produces_inactive_verdict() {
        let sqrt = crate::parser::price::price_to_sqrt_price_x64(150.0, 9, 6);
        let pool = make_pool_state(sqrt, 0);
        let pos  = make_position(-1000, 1000);

        let analysis = PositionAnalysis::compute(&[], &pos, &pool, &pool).unwrap();
        use crate::engine::verdict::VerdictLabel;
        assert_eq!(analysis.verdict.label, VerdictLabel::Inactive);
    }
}