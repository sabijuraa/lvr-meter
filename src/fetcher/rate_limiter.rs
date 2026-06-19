use std::time::{Duration, Instant};

/// Token bucket rate limiter.
/// Default: 10 requests/second (Helius free tier).
pub struct RateLimiter {
    tokens:      f64,
    max_tokens:  f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// 10 requests per second — Helius free tier limit
    pub fn helius_free_tier() -> Self {
        Self::new(10.0, 10.0)
    }

    /// Refill tokens based on elapsed time, then consume one token.
    /// If no tokens available, sleeps until one is ready.
    pub fn acquire(&mut self) {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            return;
        }

        // Calculate how long until next token is available
        let wait_secs = (1.0 - self.tokens) / self.refill_rate;
        let wait      = Duration::from_secs_f64(wait_secs);

        std::thread::sleep(wait);

        self.refill();
        self.tokens = (self.tokens - 1.0).max(0.0);
    }

    fn refill(&mut self) {
        let now     = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let added   = elapsed * self.refill_rate;

        self.tokens      = (self.tokens + added).min(self.max_tokens);
        self.last_refill = now;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_limiter_starts_full() {
        let limiter = RateLimiter::new(10.0, 10.0);
        assert_eq!(limiter.tokens, 10.0);
        assert_eq!(limiter.max_tokens, 10.0);
    }

    #[test]
    fn helius_free_tier_is_ten_rps() {
        let limiter = RateLimiter::helius_free_tier();
        assert_eq!(limiter.max_tokens, 10.0);
        assert_eq!(limiter.refill_rate, 10.0);
    }

    #[test]
    fn acquire_decrements_tokens() {
        let mut limiter = RateLimiter::new(10.0, 10.0);
        limiter.acquire();
        assert!(limiter.tokens < 10.0);
    }

    #[test]
    fn tokens_do_not_exceed_max() {
        let mut limiter = RateLimiter::new(5.0, 5.0);
        // Simulate time passing — tokens should not exceed max
        std::thread::sleep(Duration::from_millis(500));
        limiter.refill();
        assert!(limiter.tokens <= limiter.max_tokens);
    }

    #[test]
    fn ten_consecutive_acquires_complete() {
        // With 10 tokens available, 10 acquires should not sleep at all
        let mut limiter  = RateLimiter::new(10.0, 10.0);
        let start        = Instant::now();

        for _ in 0..10 {
            limiter.acquire();
        }

        // Should complete in well under 1 second
        assert!(start.elapsed() < Duration::from_millis(100));
    }
}