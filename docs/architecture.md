# Architecture

This document describes the system architecture of lvr-meter — how it is
structured, why it is structured that way, and what rules govern the
relationships between components.

---

## Purpose in one sentence

lvr-meter turns raw Solana blockchain history into a financial truth that
Raydium's dashboard hides from liquidity providers.

That sentence contains the entire architecture. "Raw blockchain history"
is the input layer. "Financial truth" is the computation layer. "Hides
from you" means the output layer must make the result legible and
actionable, not just numerically correct.

---

## The five layers

```
┌─────────────────────────────────────────┐
│  Layer 1: Input                         │
│  CLI args → validated Config            │
├─────────────────────────────────────────┤
│  Layer 2: Fetching                      │
│  Config → raw transactions + accounts   │
├─────────────────────────────────────────┤
│  Layer 3: Parsing                       │
│  Raw transactions → SwapEvent list      │
├─────────────────────────────────────────┤
│  Layer 4: Computation                   │
│  SwapEvent list → PositionAnalysis      │
├─────────────────────────────────────────┤
│  Layer 5: Output                        │
│  PositionAnalysis → terminal tables     │
└─────────────────────────────────────────┘
```

Each layer has one job. Each layer's output is the next layer's input.
No layer skips levels. No layer reaches backward into a previous layer.

---

## Layer responsibilities

### Layer 1 — Input (`src/config/`)

Owns everything the user provides at the command line. Validates all
inputs eagerly — if the wallet address is malformed, the program fails
here with a clear message before touching the network.

- **Input:** raw CLI arguments and environment variables
- **Output:** `Config` — a validated, typed representation of the user's intent
- **Key types:** `WalletAddress`, `DateRange`, `PoolFilter`, `Config`

**Rule:** this layer never makes network calls. It never reads files.
It only validates and transforms what the user provided.

---

### Layer 2 — Fetching (`src/fetcher/`)

Owns all communication with the outside world. Helius RPC calls,
Solana account reads, transaction history pagination, and the local
disk cache all live here. Nothing outside this layer makes network calls.

- **Input:** `Config`
- **Output:** `FetchResult` — raw transaction data and account states organized by pool and position
- **Key types:** `RpcClientWrapper`, `TxCache`, `RateLimiter`, `PositionInventory`, `FetchResult`

**Rule:** this layer never does financial math. It fetches and stores
data. It does not interpret what that data means.

---

### Layer 3 — Parsing (`src/parser/`)

Owns the translation from Solana's binary encoding to the financial
domain model. Decodes instruction data, extracts pre/post account
states, converts sqrtPriceX64 to float prices, filters swaps to the
position's tick range.

- **Input:** `FetchResult`
- **Output:** `Vec<SwapEvent>` — a clean, sorted, filtered list of swap events in the position's tick range
- **Key types:** `SwapEvent`, `SwapDirection`

**Rule:** this layer never makes network calls. It never does aggregate
math — it transforms one transaction at a time. The output list is
sorted by slot ascending and deduplicated.

---

### Layer 4 — Computation (`src/engine/`)

Owns all financial mathematics. LVR computation, fee reconciliation,
volatility estimation, regime classification, range efficiency, and the
position optimizer all live here. This layer is entirely pure — no I/O
of any kind.

- **Input:** `Vec<SwapEvent>` plus position metadata from `FetchResult`
- **Output:** `PositionAnalysis` — the complete analytical result
- **Key types:** `LvrResult`, `FeeResult`, `Verdict`, `RegimeResult`, `OptimizerResult`, `PositionAnalysis`

**Rule:** no network calls, no file I/O, no randomness. Given the same
inputs this layer always produces the same outputs. Every function in
this layer is unit testable in isolation.

---

### Layer 5 — Output (`src/output/`)

Owns all presentation. Formats tables, applies colors, renders the
two-section output, handles JSON serialization mode.

- **Input:** `PositionAnalysis`
- **Output:** bytes written to stdout

**Rule:** no math. No business logic. If you find yourself computing
something in the output layer, move it to the engine layer.

---

## Dependency rules

Lower layers never import from upper layers.

```
config   →  (no imports from other layers)
fetcher  →  config
parser   →  fetcher, config
engine   →  parser, config
output   →  engine, config
main     →  all layers
```

This is enforced by code review, not by the compiler. A contributor
who imports `engine` from `fetcher` is violating this rule even if
Rust allows it.

---

## Error handling strategy

There are two categories of failure in this system.

**Recoverable failures** — a single transaction cannot be parsed, one
RPC call times out, a swap event has an unexpected format. These are
logged with `tracing::warn!` and skipped. The analysis continues with
the remaining data. The final output reports how many events were
skipped and why.

**Unrecoverable failures** — the wallet has no positions in the date
range, the RPC is unreachable, the API key is missing or invalid. These
return an `Err` that propagates to `main.rs`, which prints a clean
human-readable message and exits with code 1.

The distinction matters because blockchain data is inherently noisy. A
strict fail-on-first-error policy makes the tool unusable on real
wallets.

---

## Data flow end to end

```
User types:
  lvr-meter --wallet ABC --from 2025-01-01 --to 2025-03-31
          │
          ▼
Layer 1: Parse and validate CLI args
  → Config { wallet: WalletAddress, date_range: DateRange, ... }
          │
          ▼
Layer 2: Convert dates to slot numbers
  → fetch PersonalPosition accounts for wallet
  → fetch PoolState for each pool
  → fetch swap transactions for each pool (paginated, cached)
  → FetchResult { positions, pool_states, transactions_by_pool }
          │
          ▼
Layer 3: For each transaction
  → detect Raydium swap instruction discriminator
  → extract pre/post sqrtPriceX64 and active liquidity
  → convert sqrtPriceX64 to float price
  → filter to position tick range
  → Vec<SwapEvent> sorted by slot ascending
          │
          ▼
Layer 4:
  → LVR engine: accumulate (ΔP)² / (8 × fee_rate) × L per swap
  → Fee reconciler: fee_growth_delta × liquidity / 2^128
  → Ratio: fees_usd / total_lvr_usd
  → Regime classifier: log-returns → volatility → regime label
  → Range efficiency: time-in-range / total-time
  → Optimizer: grid search over fee tiers × range widths
  → PositionAnalysis
          │
          ▼
Layer 5: Format and print
  → Section 1: historical table with verdict
  → Section 2: parameter recommendation
```

---

## Extension points

The following areas are intentionally designed for future extension.

**Adding Orca Whirlpool support** — The parser layer has a
`src/parser/raydium/` subdirectory. An `src/parser/orca/` subdirectory
with the same interface plugs in without changing any other layer.

**Adding a new chain** — The fetcher layer owns all chain-specific RPC
logic. A new chain adds a new fetcher module. The parser, engine, and
output layers are chain-agnostic once SwapEvents are produced.

**Adding real-time monitoring** — The engine layer is pure functions
over SwapEvent slices. A streaming input layer could feed events one at
a time into the engine without changing the engine at all.

**Adding a web dashboard** — The output layer currently writes to
stdout. A parallel output module writing JSON to a WebSocket connection
requires no changes to any other layer.

---

## What this system deliberately does not do

- Does not execute transactions or manage positions
- Does not run continuously or provide real-time alerts
- Does not store user data anywhere except the local cache
- Does not require a database — flat files are sufficient at this scale
- Does not have a server component — it is a local CLI tool
- Does not support chains other than Solana in version one