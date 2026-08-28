//! End-to-end integration test against Solana mainnet.
//!
//! Runs the full pipeline: config → fetch → parse → engine → optimizer → output.
//! Requires HELIUS_API_KEY to be set.
//!
//! Run with:
//!   cargo test e2e -- --ignored
//!   cargo test e2e -- --ignored --nocapture   (to see full output)

use assert_cmd::Command;
use predicates::prelude::*;

/// Known public wallet with documented Raydium CLMM positions.
const TEST_WALLET: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgHkv";
const FROM:        &str = "2025-01-01";
const TO:          &str = "2025-03-31";

fn cmd() -> Command {
    let mut c = Command::cargo_bin("lvr-meter").unwrap();
    dotenvy::dotenv().ok();
    let api_key = std::env::var("HELIUS_API_KEY")
        .expect("HELIUS_API_KEY must be set for e2e tests");
    c.env("HELIUS_API_KEY", api_key);
    c
}

// ── Dry-run smoke test (fast) ─────────────────────────────────────────────────

#[test]
#[ignore]
fn e2e_dry_run_exits_zero() {
    cmd()
        .args(["--wallet", TEST_WALLET, "--from", FROM, "--to", TO, "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("lvr-meter dry run"))
        .stdout(predicate::str::contains("7xKXtg2C"));
}

// ── JSON output tests ─────────────────────────────────────────────────────────

#[test]
#[ignore]
fn e2e_json_output_is_valid_json() {
    let output = cmd()
        .args([
            "--wallet", TEST_WALLET,
            "--from",   FROM,
            "--to",     TO,
            "--output", "json",
            "--no-cache",
        ])
        .output()
        .expect("failed to run lvr-meter");

    assert!(
        output.status.success(),
        "Process exited with non-zero code.\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout  = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout is not valid JSON");

    // Top-level keys must exist
    assert!(parsed.get("analyses").is_some(),  "Missing 'analyses' key");
    assert!(parsed.get("optimizer").is_some(), "Missing 'optimizer' key");

    println!("JSON output keys: {:?}", parsed.as_object().unwrap().keys().collect::<Vec<_>>());
}

#[test]
#[ignore]
fn e2e_json_analyses_array_is_non_empty() {
    let output = cmd()
        .args([
            "--wallet", TEST_WALLET,
            "--from",   FROM,
            "--to",     TO,
            "--output", "json",
        ])
        .output()
        .expect("failed to run lvr-meter");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout is not valid JSON");

    let analyses = parsed["analyses"].as_array().expect("analyses is not an array");
    assert!(
        !analyses.is_empty(),
        "Expected at least one position analysis"
    );

    println!("Analyses count: {}", analyses.len());
}

#[test]
#[ignore]
fn e2e_json_fee_to_lvr_ratio_is_valid() {
    let output = cmd()
        .args([
            "--wallet", TEST_WALLET,
            "--from",   FROM,
            "--to",     TO,
            "--output", "json",
        ])
        .output()
        .expect("failed to run lvr-meter");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout is not valid JSON");

    let analyses = parsed["analyses"].as_array().expect("analyses is array");
    for (i, analysis) in analyses.iter().enumerate() {
        let ratio = analysis["verdict"]["ratio"]
            .as_f64()
            .expect("ratio is not a number");

        assert!(
            ratio >= 0.0,
            "Analysis {}: ratio {} is negative",
            i, ratio
        );

        assert!(
            ratio.is_finite(),
            "Analysis {}: ratio {} is not finite",
            i, ratio
        );

        let label = analysis["verdict"]["label"]
            .as_str()
            .expect("label is not a string");

        assert!(
            ["PROFITABLE", "MARGINAL", "UNPROFITABLE", "INACTIVE"].contains(&label),
            "Unexpected label: {}",
            label
        );

        println!("Position {}: ratio={:.3} label={}", i, ratio, label);
    }
}

#[test]
#[ignore]
fn e2e_optimizer_produces_recommendation() {
    let output = cmd()
        .args([
            "--wallet", TEST_WALLET,
            "--from",   FROM,
            "--to",     TO,
            "--output", "json",
        ])
        .output()
        .expect("failed to run lvr-meter");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .expect("stdout is not valid JSON");

    let optimizer = &parsed["optimizer"];

    let fee_bps = optimizer["fee_tier_bps"]
        .as_u64()
        .expect("fee_tier_bps is not a number");
    assert!(
        [1, 5, 25, 100].contains(&(fee_bps as u16)),
        "Unexpected fee tier: {} bps",
        fee_bps
    );

    let range_pct = optimizer["range_width_pct"]
        .as_f64()
        .expect("range_width_pct is not a number");
    assert!(
        range_pct >= 1.0 && range_pct <= 25.0,
        "Range width {} out of [1.0, 25.0]",
        range_pct
    );

    let confidence = optimizer["confidence"]
        .as_str()
        .expect("confidence is not a string");
    assert!(
        ["high", "medium", "low"].contains(&confidence),
        "Unexpected confidence: {}",
        confidence
    );

    println!(
        "Optimizer: {} bps fee, ±{:.1}% range, {} confidence, ratio={:.3}",
        fee_bps,
        range_pct,
        confidence,
        optimizer["projected_ratio"].as_f64().unwrap_or(0.0)
    );
}

// ── Table output tests ────────────────────────────────────────────────────────

#[test]
#[ignore]
fn e2e_table_output_contains_section_headers() {
    cmd()
        .args(["--wallet", TEST_WALLET, "--from", FROM, "--to", TO])
        .assert()
        .success()
        .stdout(predicate::str::contains("Section 1"))
        .stdout(predicate::str::contains("Section 2"));
}

#[test]
#[ignore]
fn e2e_table_output_contains_summary() {
    cmd()
        .args(["--wallet", TEST_WALLET, "--from", FROM, "--to", TO])
        .assert()
        .success()
        .stdout(predicate::str::contains("Summary"))
        .stdout(predicate::str::contains("Positions analyzed"));
}

#[test]
#[ignore]
fn e2e_table_output_contains_recommendation() {
    cmd()
        .args(["--wallet", TEST_WALLET, "--from", FROM, "--to", TO])
        .assert()
        .success()
        .stdout(predicate::str::contains("Recommended"))
        .stdout(predicate::str::contains("fee tier"))
        .stdout(predicate::str::contains("Confidence"));
}

// ── Cache behaviour tests ─────────────────────────────────────────────────────

#[test]
#[ignore]
fn e2e_second_run_is_faster_due_to_cache() {
    use std::time::Instant;

    // First run — hits network
    let t0 = Instant::now();
    cmd()
        .args([
            "--wallet", TEST_WALLET,
            "--from",   FROM,
            "--to",     TO,
            "--no-cache",
            "--output", "json",
        ])
        .assert()
        .success();
    let first_run = t0.elapsed();

    // Second run — should hit cache
    let t1 = Instant::now();
    cmd()
        .args([
            "--wallet", TEST_WALLET,
            "--from",   FROM,
            "--to",     TO,
            "--output", "json",
        ])
        .assert()
        .success();
    let second_run = t1.elapsed();

    println!("First run:  {:.1}s", first_run.as_secs_f64());
    println!("Second run: {:.1}s", second_run.as_secs_f64());

    // Cache run should be at least 2x faster
    assert!(
        second_run < first_run / 2,
        "Second run ({:.1}s) was not significantly faster than first ({:.1}s) — cache may not be working",
        second_run.as_secs_f64(),
        first_run.as_secs_f64()
    );
}

#[test]
#[ignore]
fn e2e_no_cache_flag_produces_same_result_as_cache() {
    let run = |extra_args: &[&str]| {
        let mut args = vec![
            "--wallet", TEST_WALLET,
            "--from",   FROM,
            "--to",     TO,
            "--output", "json",
        ];
        args.extend_from_slice(extra_args);

        let output = cmd().args(&args).output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        parsed["analyses"].as_array().unwrap().len()
    };

    let count_cached   = run(&[]);
    let count_no_cache = run(&["--no-cache"]);

    assert_eq!(
        count_cached,
        count_no_cache,
        "Cache and no-cache runs produced different analysis counts: {} vs {}",
        count_cached,
        count_no_cache
    );
}

// ── Error handling tests ──────────────────────────────────────────────────────

#[test]
#[ignore]
fn e2e_invalid_wallet_exits_nonzero_with_clean_message() {
    cmd()
        .args(["--wallet", "invalid", "--from", FROM, "--to", TO])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error:"))
        .stderr(predicate::str::contains("backtrace").not());
}

#[test]
#[ignore]
fn e2e_missing_api_key_exits_with_clean_message() {
    Command::cargo_bin("lvr-meter")
        .unwrap()
        .env("HELIUS_API_KEY", "")
        .args(["--wallet", TEST_WALLET, "--from", FROM, "--to", TO])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error:"));
}