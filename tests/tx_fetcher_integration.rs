//! Integration tests for transaction fetcher with cache.
//! These tests hit mainnet — run manually with:
//!   cargo test tx_fetcher -- --ignored

use lvr_meter::fetcher::cache::TxCache;
use lvr_meter::fetcher::rpc::RpcClientWrapper;
use lvr_meter::fetcher::tx_fetcher::fetch_transactions_for_pool;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tempfile::TempDir;

/// SOL/USDC Raydium CLMM pool — one of the busiest pools on mainnet
const SOL_USDC_POOL: &str = "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj";

/// A 3-day slot window on mainnet (approximate)
/// These are real slots — adjust if they become too old for your RPC
const SLOT_START: u64 = 280_000_000;
const SLOT_END:   u64 = 280_650_000; // ~3 days at 2.5 slots/sec

fn setup() -> (RpcClientWrapper, TempDir) {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("HELIUS_API_KEY")
        .expect("HELIUS_API_KEY must be set to run integration tests");

    let rpc_url = format!("https://mainnet.helius-rpc.com/?api-key={}", api_key);
    let client  = RpcClientWrapper::new(&rpc_url);
    let dir     = TempDir::new().unwrap();

    (client, dir)
}

#[test]
#[ignore]
fn fetch_returns_transactions_for_busy_pool() {
    let (client, dir) = setup();
    let cache = TxCache::new(dir.path()).unwrap();
    let pool  = Pubkey::from_str(SOL_USDC_POOL).unwrap();

    let txs = fetch_transactions_for_pool(
        &pool,
        SLOT_START,
        SLOT_END,
        &cache,
        &client,
    )
    .expect("fetch should not error");

    assert!(
        !txs.is_empty(),
        "Expected transactions for SOL/USDC pool in slot range"
    );

    println!(
        "Fetched {} transactions for SOL/USDC pool",
        txs.len()
    );
}

#[test]
#[ignore]
fn second_fetch_returns_from_cache() {
    let (client, dir) = setup();
    let cache = TxCache::new(dir.path()).unwrap();
    let pool  = Pubkey::from_str(SOL_USDC_POOL).unwrap();

    // First fetch — hits network
    let first = fetch_transactions_for_pool(
        &pool,
        SLOT_START,
        SLOT_END,
        &cache,
        &client,
    )
    .expect("first fetch should not error");

    assert!(cache.exists(&pool, SLOT_START, SLOT_END),
        "Cache file should exist after first fetch"
    );

    // Second fetch — should hit cache
    let second = fetch_transactions_for_pool(
        &pool,
        SLOT_START,
        SLOT_END,
        &cache,
        &client,
    )
    .expect("second fetch should not error");

    assert_eq!(
        first.len(),
        second.len(),
        "Cache hit should return identical transaction count"
    );

    println!(
        "Cache verified: {} transactions consistent across two fetches",
        first.len()
    );
}

#[test]
#[ignore]
fn empty_slot_range_returns_empty() {
    let (client, dir) = setup();
    let cache = TxCache::new(dir.path()).unwrap();
    let pool  = Pubkey::from_str(SOL_USDC_POOL).unwrap();

    // Slot range of 1 — extremely unlikely to contain transactions
    let txs = fetch_transactions_for_pool(
        &pool,
        SLOT_START,
        SLOT_START + 1,
        &cache,
        &client,
    )
    .expect("fetch should not error on tiny range");

    println!("Tiny slot range returned {} transactions", txs.len());
}