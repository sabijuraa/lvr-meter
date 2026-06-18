pub struct HeliusClient {
    api_key: String,
}

impl HeliusClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_owned(),
        }
    }

    /// RPC endpoint — used by RpcClientWrapper for all Solana RPC calls
    pub fn rpc_url(&self) -> String {
        format!(
            "https://mainnet.helius-rpc.com/?api-key={}",
            self.api_key
        )
    }

    /// Enhanced transaction API — used in Phase 3 for parsed transaction data
    pub fn enhanced_tx_url(&self) -> String {
        format!(
            "https://api.helius.xyz/v0/transactions?api-key={}",
            self.api_key
        )
    }

    /// Returns the api key — needed when passing to RpcClientWrapper
    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpc_url_contains_api_key() {
        let client = HeliusClient::new("test-key-123");
        let url = client.rpc_url();
        assert!(url.contains("test-key-123"));
        assert!(url.contains("mainnet.helius-rpc.com"));
    }

    #[test]
    fn enhanced_tx_url_contains_api_key() {
        let client = HeliusClient::new("test-key-123");
        let url = client.enhanced_tx_url();
        assert!(url.contains("test-key-123"));
        assert!(url.contains("api.helius.xyz"));
    }

    #[test]
    fn different_keys_produce_different_urls() {
        let client_a = HeliusClient::new("key-aaa");
        let client_b = HeliusClient::new("key-bbb");
        assert_ne!(client_a.rpc_url(), client_b.rpc_url());
    }

    #[test]
    fn api_key_getter_returns_correct_value() {
        let client = HeliusClient::new("my-key");
        assert_eq!(client.api_key(), "my-key");
    }
}