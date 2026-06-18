//! Integration test — hits mainnet. Run with:
//!   cargo test position_fetcher -- --ignored
//!
//! Uses a known public wallet with documented Raydium CLMM positions.

use lvr_meter::config::WalletAddress;
use lvr_meter::fetcher::rpc::RpcClientWrapper;
use lvr_meter::fetcher::raydium::position_fetcher::fetch_positions;

/// Public wallet known to hold Raydium CLMM positions.
/// Verify at: https://raydium.io/portfolio/
const TEST_WALLET: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgHkv";

#[test]
#[ignore]
fn fetch_positions_returns_results_for_known_wallet() {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("HELIUS_API_KEY")
        .expect("HELIUS_API_KEY must be set");

    let rpc_url = format!("https://mainnet.helius-rpc.com/?api-key={}", api_key);
    let client  = RpcClientWrapper::new(&rpc_url);
    let wallet  = WalletAddress::parse(TEST_WALLET).unwrap();

    let positions = fetch_positions(&wallet, &client)
        .expect("fetch_positions should not error");

    assert!(!positions.is_empty(), "Expected at least one position");

    for pos in &positions {
        // tick_lower must be less than tick_upper
        assert!(
            pos.tick_lower_index < pos.tick_upper_index,
            "tick_lower {} >= tick_upper {}",
            pos.tick_lower_index,
            pos.tick_upper_index
        );

        // ticks must be within Raydium's valid range
        assert!(pos.tick_lower_index >= -443636);
        assert!(pos.tick_upper_index <=  443636);
    }

    println!("Found {} positions for wallet {}", positions.len(), TEST_WALLET);
}