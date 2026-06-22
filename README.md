# lvr-meter

![CI](https://github.com/sabijuraa/lvr-meter/actions/workflows/ci.yml/badge.svg)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

**LVR calculator and position optimizer for Solana CLMMs.**

Most LPs on Raydium are losing money without knowing it. This tool computes
the true cost of providing liquidity using Loss-Versus-Rebalancing (LVR) —
the academically correct measure — and recommends optimal fee tiers and range
widths grounded in realized volatility data from your actual positions.

---

## What is LVR?

Every time you provide liquidity in a CLMM pool, arbitrageurs extract value
from your position when the price moves. This extraction is called
Loss-Versus-Rebalancing. The fees your position earns must exceed the LVR
paid to arbitrageurs for the position to be profitable on a risk-adjusted basis.

Most LP dashboards show you fee income. None of them show you what you paid
in LVR. This tool shows you both.

---

## Features

- Fetches real on-chain Raydium CLMM positions via Helius RPC
- Computes instantaneous and cumulative LVR using the Milionis et al. formula
- Computes fees earned using Solana's fee growth accumulator mechanism
- Classifies market regime (trending, mean-reverting, volatile)
- Grid search optimizer recommends fee tier and range width
- Disk cache for transaction data — re-runs are instant
- Table output with colored verdict or JSON for scripting
- Rate limiter, retry logic, exponential backoff built in

---

## Install

```bash
git clone https://github.com/sabijuraa/lvr-meter
cd lvr-meter
cargo build --release
```

Get a free Helius API key at [helius.dev](https://helius.dev) then:

```bash
# Linux / macOS
export HELIUS_API_KEY=your-key-here

# Windows PowerShell
$env:HELIUS_API_KEY = "your-key-here"
```

---

## Usage

```bash
# Analyze all CLMM positions for a wallet
./lvr-meter --wallet <WALLET> --from 2025-01-01 --to 2025-03-31

# Validate config without hitting the network
./lvr-meter --wallet <WALLET> --from 2025-01-01 --to 2025-03-31 --dry-run

# JSON output for scripting
./lvr-meter --wallet <WALLET> --from 2025-01-01 --to 2025-03-31 --output json

# Force re-fetch, bypass cache
./lvr-meter --wallet <WALLET> --from 2025-01-01 --to 2025-03-31 --no-cache

# Filter to Raydium only
./lvr-meter --wallet <WALLET> --from 2025-01-01 --to 2025-03-31 --protocol raydium
```

