use assert_cmd::Command;
use predicates::prelude::*;

// Helper — returns a Command pointed at our binary
// with the HELIUS_API_KEY env var set so config doesn't
// fail on the missing key check
fn cmd() -> Command {
    let mut c = Command::cargo_bin("lvr-meter").unwrap();
    c.env("HELIUS_API_KEY", "test-key-for-integration-tests");
    c
}

// Valid arguments used across multiple tests
const WALLET: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgHkv";
const FROM:   &str = "2025-01-01";
const TO:     &str = "2025-03-31";

// ── Happy path ────────────────────────────────────────────────

#[test]
fn dry_run_exits_zero() {
    cmd()
        .args(["--wallet", WALLET, "--from", FROM, "--to", TO, "--dry-run"])
        .assert()
        .success();
}

#[test]
fn dry_run_shows_truncated_wallet() {
    cmd()
        .args(["--wallet", WALLET, "--from", FROM, "--to", TO, "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("7xKXtg2C....gHkv")); 
}

#[test]
fn dry_run_shows_date_range() {
    cmd()
        .args(["--wallet", WALLET, "--from", FROM, "--to", TO, "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("2025-01-01"))
        .stdout(predicate::str::contains("2025-03-31"));
}

#[test]
fn dry_run_with_protocol_raydium() {
    cmd()
        .args([
            "--wallet", WALLET,
            "--from",   FROM,
            "--to",     TO,
            "--protocol", "raydium",
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Raydium"));
}

#[test]
fn dry_run_hides_api_key() {
    cmd()
        .args(["--wallet", WALLET, "--from", FROM, "--to", TO, "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("****"))
        .stdout(predicate::str::contains("test-key-for-integration-tests").not());
}

// ── Missing required arguments ─────────────────────────────────

#[test]
fn missing_wallet_exits_nonzero() {
    cmd()
        .args(["--from", FROM, "--to", TO, "--dry-run"])
        .assert()
        .failure();
}

#[test]
fn missing_from_exits_nonzero() {
    cmd()
        .args(["--wallet", WALLET, "--to", TO, "--dry-run"])
        .assert()
        .failure();
}

#[test]
fn missing_to_exits_nonzero() {
    cmd()
        .args(["--wallet", WALLET, "--from", FROM, "--dry-run"])
        .assert()
        .failure();
}

// ── Invalid argument values ────────────────────────────────────

#[test]
fn invalid_date_format_exits_nonzero() {
    cmd()
        .args(["--wallet", WALLET, "--from", "01-01-2025", "--to", TO, "--dry-run"])
        .assert()
        .failure();
}

#[test]
fn invalid_wallet_exits_nonzero() {
    cmd()
        .args(["--wallet", "tooshort", "--from", FROM, "--to", TO, "--dry-run"])
        .assert()
        .failure();
}

#[test]
fn invalid_protocol_exits_nonzero() {
    cmd()
        .args([
            "--wallet", WALLET,
            "--from",   FROM,
            "--to",     TO,
            "--protocol", "uniswap",
            "--dry-run",
        ])
        .assert()
        .failure();
}

#[test]
fn reversed_date_range_exits_nonzero() {
    cmd()
        .args(["--wallet", WALLET, "--from", TO, "--to", FROM, "--dry-run"])
        .assert()
        .failure();
}

#[test]
fn missing_helius_key_exits_nonzero() {
    Command::cargo_bin("lvr-meter")
        .unwrap()
        .env("HELIUS_API_KEY", "")      
        .args(["--wallet", WALLET, "--from", FROM, "--to", TO, "--dry-run"])
        .assert()
        .failure();
}