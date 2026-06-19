use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, PartialEq)]
pub enum SwapDirection {
    ZeroForOne,
    OneForZero,
}

#[derive(Debug, Clone)]
pub struct SwapEvent {
    pub slot:               u64,
    pub timestamp:          i64,
    pub pool:               Pubkey,
    pub price_before:       f64,
    pub price_after:        f64,
    pub sqrt_price_before:  u128,
    pub sqrt_price_after:   u128,
    pub active_liquidity:   u128,
    pub fee_rate:           u16,
    pub direction:          SwapDirection,
}

impl SwapEvent {
    pub fn price_delta(&self) -> f64 {
        self.price_after - self.price_before
    }

    pub fn price_delta_abs(&self) -> f64 {
        self.price_delta().abs()
    }

    pub fn price_moved_up(&self) -> bool {
        self.price_after > self.price_before
    }

    pub fn fee_rate_decimal(&self) -> f64 {
        self.fee_rate as f64 / 10_000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(price_before: f64, price_after: f64, direction: SwapDirection) -> SwapEvent {
        SwapEvent {
            slot:              100,
            timestamp:         1_700_000_000,
            pool:              Pubkey::new_unique(),
            price_before,
            price_after,
            sqrt_price_before: 0,
            sqrt_price_after:  0,
            active_liquidity:  1_000_000,
            fee_rate:          25,
            direction,
        }
    }

    #[test]
    fn price_delta_positive_when_price_rises() {
        let e = make_event(100.0, 101.0, SwapDirection::OneForZero);
        assert!((e.price_delta() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn price_delta_negative_when_price_falls() {
        let e = make_event(100.0, 99.0, SwapDirection::ZeroForOne);
        assert!((e.price_delta() - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn price_delta_abs_always_positive() {
        let up   = make_event(100.0, 101.0, SwapDirection::OneForZero);
        let down = make_event(100.0,  99.0, SwapDirection::ZeroForOne);
        assert!(up.price_delta_abs()   > 0.0);
        assert!(down.price_delta_abs() > 0.0);
    }

    #[test]
    fn price_moved_up_correct() {
        let up   = make_event(100.0, 101.0, SwapDirection::OneForZero);
        let down = make_event(100.0,  99.0, SwapDirection::ZeroForOne);
        assert!( up.price_moved_up());
        assert!(!down.price_moved_up());
    }

    #[test]
    fn fee_rate_decimal_25bps() {
        let e = make_event(100.0, 101.0, SwapDirection::OneForZero);
        assert!((e.fee_rate_decimal() - 0.0025).abs() < 1e-10);
    }

    #[test]
    fn fee_rate_decimal_100bps() {
        let mut e = make_event(100.0, 101.0, SwapDirection::OneForZero);
        e.fee_rate = 100;
        assert!((e.fee_rate_decimal() - 0.01).abs() < 1e-10);
    }

    #[test]
    fn direction_variants_are_distinct() {
        assert_ne!(SwapDirection::ZeroForOne, SwapDirection::OneForZero);
    }

    #[test]
    fn clone_produces_equal_event() {
        let e      = make_event(150.0, 151.0, SwapDirection::OneForZero);
        let cloned = e.clone();
        assert_eq!(e.slot,             cloned.slot);
        assert_eq!(e.price_before,     cloned.price_before);
        assert_eq!(e.price_after,      cloned.price_after);
        assert_eq!(e.active_liquidity, cloned.active_liquidity);
        assert_eq!(e.direction,        cloned.direction);
    }
}