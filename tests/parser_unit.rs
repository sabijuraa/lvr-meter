//! Parser unit tests using real transaction fixtures.
//!
//! Fixtures are real Raydium CLMM swap transactions saved as JSON.
//! Run fixture fetch script first:
//!   ./scripts/fetch_fixtures.sh
//!
//! These tests run without network access — pure deserialization + parsing.

use lvr_meter::fetcher::raydium::pool_state::PoolState;
use lvr_meter::fetcher::raydium::types::PersonalPositionState;
use lvr_meter::parser::batch::parse_pool_transactions;
use lvr_meter::parser::filter::tick_to_price;
use lvr_meter::parser::price::sqrt_price_x64_to_price;
use lvr_meter::parser::raydium::discriminator::{
    SWAP_V1_DISCRIMINATOR, SWAP_V2_DISCRIMINATOR,
};
use lvr_meter::parser::types::SwapDirection;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedTransactionWithStatusMeta;
use std::fs;
use std::path::Path;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn load_fixture(filename: &str) -> Option<EncodedTransactionWithStatusMeta> {
    let path = Path::new("tests/fixtures").join(filename);
    if !path.exists() {
        return None;
    }
    let json = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&json).ok()
}

fn list_fixture_files() -> Vec<String> {
    let dir = Path::new("tests/fixtures");
    if !dir.exists() {
        return vec![];
    }
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect()
}

fn make_sol_usdc_pool_state() -> PoolState {
    use lvr_meter::fetcher::raydium::pool_state::POOL_STATE_DISCRIMINATOR;

    // SOL/USDC pool: decimals_0=9 (SOL), decimals_1=6 (USDC), tick_spacing=1
    let sqrt_price = lvr_meter::parser::price::price_to_sqrt_price_x64(150.0, 9, 6);
    let mut data   = vec![0u8; 512];

    data[..8].copy_from_slice(&POOL_STATE_DISCRIMINATOR);
    data[8 + 225] = 9;  // decimals_0 = SOL
    data[8 + 226] = 6;  // decimals_1 = USDC
    data[8 + 227..8 + 229].copy_from_slice(&1u16.to_le_bytes()); // tick_spacing
    data[8 + 229..8 + 245].copy_from_slice(&5_000_000_000u128.to_le_bytes());
    data[8 + 245..8 + 261].copy_from_slice(&sqrt_price.to_le_bytes());
    data[8 + 261..8 + 265].copy_from_slice(&0i32.to_le_bytes());
    data[8 + 269..8 + 285].copy_from_slice(&0u128.to_le_bytes());
    data[8 + 285..8 + 301].copy_from_slice(&0u128.to_le_bytes());

    PoolState::from_account_data(&data).unwrap()
}

fn make_position(tick_lower: i32, tick_upper: i32) -> PersonalPositionState {
    PersonalPositionState {
        bump:                         [255],
        nft_mint:                     Pubkey::new_unique(),
        pool_id:                      Pubkey::new_unique(),
        tick_lower_index:             tick_lower,
        tick_upper_index:             tick_upper,
        liquidity:                    1_000_000_000,
        fee_growth_inside_0_last_x64: 0,
        fee_growth_inside_1_last_x64: 0,
        token_fees_owed_0:            0,
        token_fees_owed_1:            0,
        reward_infos:                 Default::default(),
        recent_epoch:                 0,
        padding:                      [0; 7],
    }
}

// ── Discriminator Tests ───────────────────────────────────────────────────────

#[test]
fn swap_v2_discriminator_is_correct_sha256() {
    // sha256("global:swap_v2")[:8] = [0x2b, 0x04, 0xed, 0x0b, 0x1a, 0xc9, 0x1e, 0x62]
    assert_eq!(
        SWAP_V2_DISCRIMINATOR,
        [0x2b, 0x04, 0xed, 0x0b, 0x1a, 0xc9, 0x1e, 0x62]
    );
}

#[test]
fn swap_v1_discriminator_is_correct_sha256() {
    // sha256("global:swap")[:8] = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8]
    assert_eq!(
        SWAP_V1_DISCRIMINATOR,
        [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8]
    );
}

// ── Price Conversion Tests ────────────────────────────────────────────────────

#[test]
fn price_conversion_roundtrip_sol_usdc() {
    use lvr_meter::parser::price::price_to_sqrt_price_x64;

    let prices = [50.0, 100.0, 150.0, 200.0, 300.0];
    for &p in &prices {
        let sqrt  = price_to_sqrt_price_x64(p, 9, 6);
        let back  = sqrt_price_x64_to_price(sqrt, 9, 6);
        let error = ((back - p) / p).abs();
        assert!(
            error < 0.0001,
            "Price {} roundtrip error {:.4}%",
            p, error * 100.0
        );
    }
}

#[test]
fn tick_to_price_monotonically_increases() {
    let ticks = [-1000, -500, -100, 0, 100, 500, 1000];
    let prices: Vec<f64> = ticks.iter().map(|&t| tick_to_price(t)).collect();

    for i in 1..prices.len() {
        assert!(
            prices[i] > prices[i - 1],
            "tick_to_price should be monotonically increasing"
        );
    }
}

// ── Fixture-Based Tests ───────────────────────────────────────────────────────

#[test]
fn fixtures_directory_exists_or_skip() {
    let dir = Path::new("tests/fixtures");
    if !dir.exists() {
        println!("SKIP: tests/fixtures not found — run ./scripts/fetch_fixtures.sh");
        return;
    }
    let files = list_fixture_files();
    println!("Found {} fixture files", files.len());
}

#[test]
fn all_fixtures_deserialize_without_error() {
    let files = list_fixture_files();
    if files.is_empty() {
        println!("SKIP: no fixture files found");
        return;
    }

    for file in &files {
        let tx = load_fixture(file);
        assert!(
            tx.is_some(),
            "Fixture {} failed to deserialize",
            file
        );
    }

    println!("All {} fixtures deserialized successfully", files.len());
}

#[test]
fn swap_events_have_valid_field_ranges() {
    let files = list_fixture_files();
    if files.is_empty() {
        println!("SKIP: no fixture files found");
        return;
    }

    let pool       = Pubkey::new_unique();
    let pool_state = make_sol_usdc_pool_state();
    let position   = make_position(-10000, 10000); // wide range

    let txs: Vec<EncodedTransactionWithStatusMeta> = files
        .iter()
        .filter_map(|f| load_fixture(f))
        .collect();

    let events = parse_pool_transactions(&txs, &pool, &pool_state, &position);

    for event in &events {
        // Slot must be positive
        assert!(event.slot > 0, "slot should be positive");

        // Prices must be positive
        assert!(event.price_before > 0.0, "price_before must be positive");
        assert!(event.price_after  > 0.0, "price_after must be positive");

        // For SOL/USDC, prices should be in a sane range (1–10000 USDC/SOL)
        assert!(event.price_before < 10_000.0, "price_before unreasonably large");
        assert!(event.price_after  < 10_000.0, "price_after unreasonably large");

        // Liquidity must be positive
        assert!(event.active_liquidity > 0, "liquidity must be positive");

        // sqrt prices must be consistent with direction
        match event.direction {
            SwapDirection::ZeroForOne => {
                assert!(
                    event.sqrt_price_after <= event.sqrt_price_before,
                    "ZeroForOne should have decreasing sqrt price"
                );
            }
            SwapDirection::OneForZero => {
                assert!(
                    event.sqrt_price_after >= event.sqrt_price_before,
                    "OneForZero should have increasing sqrt price"
                );
            }
        }
    }

    println!("Validated {} swap events from {} fixtures", events.len(), files.len());
}

#[test]
fn events_are_sorted_by_slot_ascending() {
    let files = list_fixture_files();
    if files.is_empty() {
        println!("SKIP: no fixture files found");
        return;
    }

    let pool       = Pubkey::new_unique();
    let pool_state = make_sol_usdc_pool_state();
    let position   = make_position(-10000, 10000);

    let txs: Vec<EncodedTransactionWithStatusMeta> = files
        .iter()
        .filter_map(|f| load_fixture(f))
        .collect();

    let events = parse_pool_transactions(&txs, &pool, &pool_state, &position);

    for i in 1..events.len() {
        assert!(
            events[i].slot >= events[i - 1].slot,
            "Events not sorted: slot {} before slot {}",
            events[i - 1].slot,
            events[i].slot
        );
    }
}

#[test]
fn no_duplicate_events_in_output() {
    let files = list_fixture_files();
    if files.is_empty() {
        println!("SKIP: no fixture files found");
        return;
    }

    let pool       = Pubkey::new_unique();
    let pool_state = make_sol_usdc_pool_state();
    let position   = make_position(-10000, 10000);

    // Feed each fixture twice — duplicates should be removed
    let txs: Vec<EncodedTransactionWithStatusMeta> = files
        .iter()
        .chain(files.iter())
        .filter_map(|f| load_fixture(f))
        .collect();

    let events_double = parse_pool_transactions(&txs, &pool, &pool_state, &position);

    let txs_single: Vec<EncodedTransactionWithStatusMeta> = files
        .iter()
        .filter_map(|f| load_fixture(f))
        .collect();

    let events_single = parse_pool_transactions(&txs_single, &pool, &pool_state, &position);

    assert_eq!(
        events_double.len(),
        events_single.len(),
        "Deduplication failed: double input produced {} events, single produced {}",
        events_double.len(),
        events_single.len()
    );
}