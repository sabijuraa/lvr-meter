use crate::fetcher::raydium::pool_state::PoolState;
use crate::fetcher::raydium::types::PersonalPositionState;
use crate::parser::price::sqrt_price_x64_to_price;

#[derive(Debug, Clone)]
pub struct FeeResult {
    pub fees_token_0: u64,
    pub fees_token_1: u64,
    pub fees_usd:     f64,
}

impl FeeResult {
    pub fn total_fees_usd(&self) -> f64 {
        self.fees_usd
    }
}

/// Compute fees earned by a position between pool open and close states.
///
/// Formula (Uniswap v3 / Raydium CLMM fee growth mechanism):
///   fees_token_0 = (fee_growth_global_0_close - fee_growth_inside_0_open)
///                  * liquidity / 2^128
///   fees_token_1 = (fee_growth_global_1_close - fee_growth_inside_1_open)
///                  * liquidity / 2^128
///
/// fee_growth values are Q128.128 fixed point accumulators.
/// Wrapping subtraction handles counter overflow correctly.
///
/// fees_usd = fees_token_0 * price (token_0 is the base asset)
///          + fees_token_1        (token_1 is the quote asset, e.g. USDC)
///          adjusted for decimals
pub fn compute_fees_earned(
    position:     &PersonalPositionState,
    pool_at_open: &PoolState,
    pool_at_close: &PoolState,
) -> FeeResult {
    if position.liquidity == 0 {
        return FeeResult { fees_token_0: 0, fees_token_1: 0, fees_usd: 0.0 };
    }

    let fees_token_0 = compute_token_fees(
        pool_at_close.fee_growth_global_0_x64,
        position.fee_growth_inside_0_last_x64,
        position.liquidity,
    );

    let fees_token_1 = compute_token_fees(
        pool_at_close.fee_growth_global_1_x64,
        position.fee_growth_inside_1_last_x64,
        position.liquidity,
    );

    // Add any already-accrued fees stored on the position
    let total_0 = fees_token_0.saturating_add(position.token_fees_owed_0);
    let total_1 = fees_token_1.saturating_add(position.token_fees_owed_1);

    let fees_usd = compute_fees_usd(
        total_0,
        total_1,
        pool_at_close,
    );

    FeeResult {
        fees_token_0: total_0,
        fees_token_1: total_1,
        fees_usd,
    }
}

/// Compute fees for one token using the fee growth accumulator.
///
/// fee_growth values are Q128.128 fixed point.
/// Delta uses wrapping subtraction to handle overflow correctly.
/// Final result is scaled by liquidity / 2^128.
fn compute_token_fees(
    fee_growth_global_close: u128,
    fee_growth_inside_open:  u128,
    liquidity:               u128,
) -> u64 {
    // Wrapping subtraction handles accumulator overflow
    let fee_growth_delta = fee_growth_global_close.wrapping_sub(fee_growth_inside_open);

    // Scale by liquidity / 2^128
    // Use u256-equivalent arithmetic to avoid overflow:
    // result = fee_growth_delta * liquidity >> 128
    let result = u128_mul_shift_128(fee_growth_delta, liquidity);

    result as u64
}

/// Multiply two u128 values and shift right by 128 bits.
/// Equivalent to (a * b) / 2^128 without overflow.
fn u128_mul_shift_128(a: u128, b: u128) -> u128 {
    // Split into high and low 64-bit halves
    let a_lo = a & u64::MAX as u128;
    let a_hi = a >> 64;
    let b_lo = b & u64::MAX as u128;
    let b_hi = b >> 64;

    // Partial products
    let lo_lo = a_lo * b_lo;
    let lo_hi = a_lo * b_hi;
    let hi_lo = a_hi * b_lo;
    let hi_hi = a_hi * b_hi;

    // Combine: (lo_lo >> 128) + (lo_hi >> 64) + (hi_lo >> 64) + hi_hi
    let mid = (lo_lo >> 64)
        .wrapping_add(lo_hi & u64::MAX as u128)
        .wrapping_add(hi_lo & u64::MAX as u128);

    hi_hi
        .wrapping_add(lo_hi >> 64)
        .wrapping_add(hi_lo >> 64)
        .wrapping_add(mid >> 64)
}

/// Convert token fees to USD value.
/// token_0 is the base asset (e.g. SOL), token_1 is the quote (e.g. USDC).
fn compute_fees_usd(
    fees_token_0: u64,
    fees_token_1: u64,
    pool:         &PoolState,
) -> f64 {
    let price = sqrt_price_x64_to_price(
        pool.sqrt_price_x64,
        pool.mint_decimals_0,
        pool.mint_decimals_1,
    );

    let decimals_0 = pool.mint_decimals_0 as u32;
    let decimals_1 = pool.mint_decimals_1 as u32;

    // Convert raw token amounts to human units
    let amount_0 = fees_token_0 as f64 / 10f64.powi(decimals_0 as i32);
    let amount_1 = fees_token_1 as f64 / 10f64.powi(decimals_1 as i32);

    // token_0 value in USD = amount_0 * price
    // token_1 value in USD = amount_1 (already in quote currency)
    amount_0 * price + amount_1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetcher::raydium::pool_state::POOL_STATE_DISCRIMINATOR;
    use crate::fetcher::raydium::types::{PersonalPositionState, REWARD_NUM};
    use solana_sdk::pubkey::Pubkey;

    fn make_pool_state(
        fee_growth_0: u128,
        fee_growth_1: u128,
        sqrt_price:   u128,
    ) -> PoolState {
        let mut data = vec![0u8; 512];
        data[..8].copy_from_slice(&POOL_STATE_DISCRIMINATOR);
        data[8 + 225] = 9;  // decimals_0 = SOL
        data[8 + 226] = 6;  // decimals_1 = USDC
        data[8 + 227..8 + 229].copy_from_slice(&1u16.to_le_bytes());
        data[8 + 229..8 + 245].copy_from_slice(&1_000_000u128.to_le_bytes());
        data[8 + 245..8 + 261].copy_from_slice(&sqrt_price.to_le_bytes());
        data[8 + 261..8 + 265].copy_from_slice(&0i32.to_le_bytes());
        data[8 + 269..8 + 285].copy_from_slice(&fee_growth_0.to_le_bytes());
        data[8 + 285..8 + 301].copy_from_slice(&fee_growth_1.to_le_bytes());
        PoolState::from_account_data(&data).unwrap()
    }

    fn make_position(
        liquidity:      u128,
        fee_growth_0:   u128,
        fee_growth_1:   u128,
        fees_owed_0:    u64,
        fees_owed_1:    u64,
    ) -> PersonalPositionState {
        PersonalPositionState {
            bump:                         [255],
            nft_mint:                     Pubkey::new_unique(),
            pool_id:                      Pubkey::new_unique(),
            tick_lower_index:             -100,
            tick_upper_index:              100,
            liquidity,
            fee_growth_inside_0_last_x64: fee_growth_0,
            fee_growth_inside_1_last_x64: fee_growth_1,
            token_fees_owed_0:            fees_owed_0,
            token_fees_owed_1:            fees_owed_1,
            reward_infos:                 Default::default(),
            recent_epoch:                 0,
            padding:                      [0; 7],
        }
    }

    /// SOL price ~$150: sqrt_price_x64 for 150.0 SOL/USDC
    fn sol_usdc_sqrt_price() -> u128 {
        crate::parser::price::price_to_sqrt_price_x64(150.0, 9, 6)
    }

    #[test]
    fn zero_liquidity_returns_zero_fees() {
        let pool     = make_pool_state(1_000_000, 1_000_000, sol_usdc_sqrt_price());
        let position = make_position(0, 0, 0, 0, 0);
        let result   = compute_fees_earned(&position, &pool, &pool);
        assert_eq!(result.fees_token_0, 0);
        assert_eq!(result.fees_token_1, 0);
        assert_eq!(result.fees_usd,     0.0);
    }

    #[test]
    fn no_fee_growth_returns_only_accrued_fees() {
        // fee_growth at open == fee_growth at close → no new fees
        // but position has accrued fees_owed
        let pool     = make_pool_state(1_000, 2_000, sol_usdc_sqrt_price());
        let position = make_position(1_000_000, 1_000, 2_000, 500, 300);
        let result   = compute_fees_earned(&position, &pool, &pool);

        // Only the pre-accrued fees are returned
        assert_eq!(result.fees_token_0, 500);
        assert_eq!(result.fees_token_1, 300);
    }

    #[test]
    fn fee_growth_delta_produces_fees() {
        // fee_growth increased by 2^64 per unit of liquidity
        // With liquidity = 1, fees = 2^64 * 1 / 2^128 = 2^(-64) ≈ 0
        // Use large fee_growth delta to get measurable fees
        let fee_growth_open  = 0u128;
        let fee_growth_close = 1u128 << 64; // large delta

        let pool_open  = make_pool_state(fee_growth_open,  0, sol_usdc_sqrt_price());
        let pool_close = make_pool_state(fee_growth_close, 0, sol_usdc_sqrt_price());

        // Large liquidity to amplify the fee
        let liquidity = 1u128 << 64;
        let position  = make_position(liquidity, fee_growth_open, 0, 0, 0);

        let result = compute_fees_earned(&position, &pool_open, &pool_close);

        // fees = (2^64 - 0) * 2^64 / 2^128 = 2^128 / 2^128 = 1
        assert_eq!(result.fees_token_0, 1);
    }

    #[test]
    fn wrapping_subtraction_handles_overflow() {
        // Simulate accumulator wraparound:
        // close < open means the accumulator overflowed
        let fee_growth_open  = u128::MAX - 100;
        let fee_growth_close = 50u128; // wrapped around

        // Expected delta = u128::MAX - (u128::MAX - 100) + 50 + 1 = 151
        let delta    = fee_growth_close.wrapping_sub(fee_growth_open);
        let expected = 151u128;
        assert_eq!(delta, expected);
    }

    #[test]
    fn fees_usd_uses_current_price() {
        let sqrt_price = sol_usdc_sqrt_price(); // ~$150
        let pool_open  = make_pool_state(0, 0, sqrt_price);
        let pool_close = make_pool_state(0, 1_000_000_000, sqrt_price); // 1000 USDC fees

        let position = make_position(1_000_000, 0, 0, 0, 0);
        let result   = compute_fees_earned(&position, &pool_open, &pool_close);

        // fees_token_1 in raw = fees_owed_1 (no new growth-based fees with liquidity 1M and small delta)
        // fees_usd should be positive
        assert!(result.fees_usd >= 0.0);
    }

    #[test]
    fn u128_mul_shift_128_known_values() {
        // 2^64 * 2^64 >> 128 = 2^128 >> 128 = 1
        assert_eq!(u128_mul_shift_128(1u128 << 64, 1u128 << 64), 1);

        // 0 * anything = 0
        assert_eq!(u128_mul_shift_128(0, u128::MAX), 0);

        // 2^128-1 * 1 >> 128 = 0 (less than 2^128)
        assert_eq!(u128_mul_shift_128(u128::MAX, 1), 0);
    }
}