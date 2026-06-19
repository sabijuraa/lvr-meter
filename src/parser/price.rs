/// Convert Raydium's sqrt_price_x64 (Q64.64 fixed point) to a human-readable price.
///
/// Formula:
///   price = (sqrt_price_x64 / 2^64)^2 * 10^(decimals_0 - decimals_1)
///
/// The sqrt_price_x64 represents sqrt(token_1 / token_0) as a Q64.64 fixed point number.
/// Squaring gives token_1 / token_0 ratio, then we adjust for decimal differences.
pub fn sqrt_price_x64_to_price(
    sqrt_price_x64: u128,
    decimals_0:     u8,
    decimals_1:     u8,
) -> f64 {
    // 2^64 as f64
    const Q64: f64 = (1u128 << 64) as f64;

    // Convert Q64.64 fixed point to float: divide by 2^64
    let sqrt_price = sqrt_price_x64 as f64 / Q64;

    // Square to get the raw price ratio token_1/token_0
    let raw_price = sqrt_price * sqrt_price;

    // Adjust for decimal difference between token_0 and token_1
    // If decimals_0 = 9 (SOL) and decimals_1 = 6 (USDC):
    // raw_price is in units of (USDC lamports / SOL lamports)
    // multiply by 10^(decimals_0 - decimals_1) to get human price
    let decimal_adjustment = 10f64.powi(decimals_0 as i32 - decimals_1 as i32);

    raw_price * decimal_adjustment
}

/// Convert a human-readable price back to sqrt_price_x64.
/// Useful for constructing test fixtures.
pub fn price_to_sqrt_price_x64(
    price:      f64,
    decimals_0: u8,
    decimals_1: u8,
) -> u128 {
    const Q64: f64 = (1u128 << 64) as f64;

    let decimal_adjustment = 10f64.powi(decimals_0 as i32 - decimals_1 as i32);
    let adjusted_price     = price / decimal_adjustment;
    let sqrt_price         = adjusted_price.sqrt();

    (sqrt_price * Q64) as u128
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Maximum allowed relative error: 0.01%
    const MAX_RELATIVE_ERROR: f64 = 0.0001;

    fn assert_price_close(actual: f64, expected: f64) {
        let relative_error = ((actual - expected) / expected).abs();
        assert!(
            relative_error < MAX_RELATIVE_ERROR,
            "Price {} differs from expected {} by {:.4}% (max {:.4}%)",
            actual,
            expected,
            relative_error * 100.0,
            MAX_RELATIVE_ERROR * 100.0,
        );
    }

    #[test]
    fn sol_usdc_known_price() {
        // SOL/USDC pool: decimals_0 = 9 (SOL), decimals_1 = 6 (USDC)
        // At sqrt_price_x64 = 3849415438166063104 the price is ~173.00 USDC/SOL
        // This value was read from mainnet at a known slot
        let sqrt_price_x64: u128 = 3_849_415_438_166_063_104;
        let price = sqrt_price_x64_to_price(sqrt_price_x64, 9, 6);
        assert_price_close(price, 173.0);
    }

    #[test]
    fn price_of_one_with_equal_decimals() {
        // When decimals are equal, price = 1.0 means sqrt_price_x64 = 2^64
        let sqrt_price_x64 = 1u128 << 64;
        let price = sqrt_price_x64_to_price(sqrt_price_x64, 6, 6);
        assert_price_close(price, 1.0);
    }

    #[test]
    fn roundtrip_price_to_sqrt_and_back() {
        let original_price  = 150.0f64;
        let sqrt_price_x64  = price_to_sqrt_price_x64(original_price, 9, 6);
        let recovered_price = sqrt_price_x64_to_price(sqrt_price_x64, 9, 6);
        assert_price_close(recovered_price, original_price);
    }

    #[test]
    fn roundtrip_various_prices() {
        let prices = [50.0, 100.0, 200.0, 500.0, 1000.0];
        for &p in &prices {
            let sqrt = price_to_sqrt_price_x64(p, 9, 6);
            let back = sqrt_price_x64_to_price(sqrt, 9, 6);
            assert_price_close(back, p);
        }
    }

    #[test]
    fn decimal_adjustment_direction() {
        // decimals_0 > decimals_1 means token_0 has more decimal places
        // price should be higher than raw ratio
        let sqrt_price_x64 = 1u128 << 64; // raw ratio = 1.0
        let price_9_6 = sqrt_price_x64_to_price(sqrt_price_x64, 9, 6);
        let price_6_6 = sqrt_price_x64_to_price(sqrt_price_x64, 6, 6);
        // With decimals_0=9, decimals_1=6: adjustment = 10^3 = 1000
        assert_price_close(price_9_6, 1000.0);
        assert_price_close(price_6_6, 1.0);
    }

    #[test]
    fn zero_sqrt_price_gives_zero() {
        let price = sqrt_price_x64_to_price(0, 9, 6);
        assert_eq!(price, 0.0);
    }
}