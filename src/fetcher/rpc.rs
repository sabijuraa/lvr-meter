use anyhow::{anyhow, Result};
use solana_client::{client_error::ClientError, rpc_client::RpcClient};
use solana_sdk::commitment_config::CommitmentConfig;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tracing::{info, warn};

use crate::fetcher::rate_limiter::RateLimiter;

pub struct RpcClientWrapper {
    pub client:      RpcClient,
    pub max_retries: u32,
    pub timeout_ms:  u64,
    rate_limiter:    Mutex<RateLimiter>,
}

impl RpcClientWrapper {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
            max_retries:  5,
            timeout_ms:   500,
            rate_limiter: Mutex::new(RateLimiter::helius_free_tier()),
        }
    }

    pub fn with_retries(rpc_url: &str, max_retries: u32, timeout_ms: u64) -> Self {
        Self {
            client: RpcClient::new_with_commitment(
                rpc_url.to_string(),
                CommitmentConfig::confirmed(),
            ),
            max_retries,
            timeout_ms,
            rate_limiter: Mutex::new(RateLimiter::helius_free_tier()),
        }
    }

    pub fn call_with_retry<T, F>(&self, label: &str, f: F) -> Result<T>
    where
        F: Fn() -> Result<T, ClientError>,
    {
        self.rate_limiter
            .lock()
            .expect("rate limiter mutex poisoned")
            .acquire();

        let mut last_err = None;

        for attempt in 0..=self.max_retries {
            match f() {
                Ok(value) => {
                    if attempt > 0 {
                        info!("{} succeeded on attempt {}", label, attempt + 1);
                    }
                    return Ok(value);
                }
                Err(e) => {
                    if !is_retryable(&e) {
                        return Err(anyhow!("{} failed (non-retryable): {}", label, e));
                    }

                    let wait_ms = self.timeout_ms * 2_u64.pow(attempt);
                    warn!(
                        "{} failed (attempt {}/{}): {}. Retrying in {}ms...",
                        label,
                        attempt + 1,
                        self.max_retries + 1,
                        e,
                        wait_ms,
                    );

                    last_err = Some(e);

                    if attempt < self.max_retries {
                        thread::sleep(Duration::from_millis(wait_ms));
                    }
                }
            }
        }

        Err(anyhow!(
            "{} failed after {} attempts: {}",
            label,
            self.max_retries + 1,
            last_err.unwrap()
        ))
    }
}

fn is_retryable(err: &ClientError) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("429")
        || msg.contains("timeout")
        || msg.contains("timed out")
        || msg.contains("connection")
        || msg.contains("503")
        || msg.contains("too many requests")
        || msg.contains("rate limit")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_wrapper_has_correct_defaults() {
        let wrapper = RpcClientWrapper::new("https://api.mainnet-beta.solana.com");
        assert_eq!(wrapper.max_retries, 5);
        assert_eq!(wrapper.timeout_ms, 500);
    }

    #[test]
    fn with_retries_sets_custom_values() {
        let wrapper = RpcClientWrapper::with_retries(
            "https://api.mainnet-beta.solana.com",
            3,
            1000,
        );
        assert_eq!(wrapper.max_retries, 3);
        assert_eq!(wrapper.timeout_ms, 1000);
    }

    #[test]
    fn call_with_retry_returns_ok_immediately() {
        let wrapper   = RpcClientWrapper::new("https://api.mainnet-beta.solana.com");
        let mut count = 0;
        let result    = wrapper.call_with_retry("test", || {
            count += 1;
            Ok::<u32, ClientError>(42)
        });
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(count, 1);
    }

    #[test]
    fn retryable_error_detection() {
        let messages = vec![
            "HTTP 429 Too Many Requests",
            "connection refused",
            "timeout waiting for response",
            "503 service unavailable",
            "rate limit exceeded",
        ];
        for msg in messages {
            let lower    = msg.to_lowercase();
            let retryable = lower.contains("429")
                || lower.contains("timeout")
                || lower.contains("connection")
                || lower.contains("503")
                || lower.contains("rate limit");
            assert!(retryable, "Expected {:?} to be retryable", msg);
        }
    }
}