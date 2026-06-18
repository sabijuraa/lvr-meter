use anyhow::{bail, Result};

const BASE58_ALPHABET: &str =
    "123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

pub struct WalletAddress(String);

impl WalletAddress {
    pub fn parse(s: &str) -> Result<Self> {
        if !(32..=44).contains(&s.len()) {
            bail!("Invalid wallet address length {}: must be 32..=44", s.len());
        }
        if let Some(bad) = s.chars().find(|c| !BASE58_ALPHABET.contains(*c)) {
            bail!("Invalid character in wallet address: {:?}", bad);
        }
        Ok(Self(s.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_wallet_address() {
        let addr = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgHkv";
        let wallet = WalletAddress::parse(addr).unwrap();
        assert_eq!(wallet.as_str(), addr);
    }

    #[test]
    fn too_short_is_err() {
        assert!(WalletAddress::parse("tooshort").is_err());
    }

    #[test]
    fn zero_char_is_err() {
        assert!(WalletAddress::parse("0000000000000000000000000000000000").is_err());
    }
}