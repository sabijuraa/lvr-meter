use anyhow::{bail, Result};
use std::str::FromStr;

const BASE58_ALPHABET: &str =
    "123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

#[derive(Debug, Clone, PartialEq)]
pub enum Protocol {
    Raydium,
    Orca,
    Both,
}

impl FromStr for Protocol {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "raydium" => Ok(Protocol::Raydium),
            "orca"    => Ok(Protocol::Orca),
            "both"    => Ok(Protocol::Both),
            _         => bail!("Unknown protocol {:?}. Valid options: raydium, orca, both", s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolFilter {
    pub protocol:      Protocol,
    pub specific_pool: Option<String>,
}

impl PoolFilter {
    pub fn new(protocol: Protocol, specific_pool: Option<String>) -> Result<Self> {
        if let Some(ref pool) = specific_pool {
            if !(32..=44).contains(&pool.len())
                || !pool.chars().all(|c| BASE58_ALPHABET.contains(c))
            {
                bail!("Invalid pool address: {:?}", pool);
            }
        }
        Ok(Self { protocol, specific_pool })
    }
#[allow(dead_code)]
    pub fn default() -> Self {
        Self {
            protocol:      Protocol::Both,
            specific_pool: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_parses_case_insensitive() {
        assert_eq!("raydium".parse::<Protocol>().unwrap(), Protocol::Raydium);
        assert_eq!("ORCA".parse::<Protocol>().unwrap(),    Protocol::Orca);
        assert_eq!("both".parse::<Protocol>().unwrap(),    Protocol::Both);
    }

    #[test]
    fn unknown_protocol_fails() {
        assert!("uniswap".parse::<Protocol>().is_err());
    }

    #[test]
    fn pool_filter_no_pool_passes() {
        assert!(PoolFilter::new(Protocol::Raydium, None).is_ok());
    }

    #[test]
    fn pool_filter_valid_pool_passes() {
        let addr = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgHkv".to_string();
        assert!(PoolFilter::new(Protocol::Both, Some(addr)).is_ok());
    }

    #[test]
    fn pool_filter_invalid_pool_fails() {
        assert!(PoolFilter::new(Protocol::Both, Some("bad".to_string())).is_err());
    }
}