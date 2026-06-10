# Data Model

This document describes every significant type in lvr-meter — what it
represents, where it comes from, what invariants it carries, and which
layer owns it.

---

## Unit conventions

Every numeric field in this codebase has an explicit unit. This rule
exists because unit confusion is the most common source of silent bugs
in financial software.

| Convention | Meaning |
|------------|---------|
| `_usd` suffix | US dollars as f64 |
| `_lamports` suffix | Raw Solana lamports as u64 |
| `_bps` suffix | Basis points as u16 — 100 bps = 1% |
| `_x64` suffix | Q64.64 fixed-point — value multiplied by 2^64, stored as u128 |
| `_pct` suffix | Fraction between 0.0 and 1.0 — not 0 to 100 |
| `_slot` suffix | Solana slot number as u64 |
| `_ts` suffix | Unix timestamp in seconds as i64 |

When a field name does not carry a unit suffix, the unit is documented
in the field's doc comment. No numeric field is ever unit-ambiguous.

---

## Layer 1 — Config types

### `WalletAddress`

```
WalletAddress(String)
```

A validated Solana public key in base58 encoding. Constructed only via
`WalletAddress::parse` which validates base58 encoding and 32-byte
decoded length. Raw strings are never used as wallet addresses anywhere
in the codebase.

**Invariants:**
- Always valid base58
- Always decodes to exactly 32 bytes
- Never empty

---

### `DateRange`

```
DateRange {
    from: NaiveDate,
    to:   NaiveDate,
}
```

A validated calendar date range for the analysis window.

**Invariants:**
- `from` is always strictly before `to`
- `to` is never in the future
- Range never exceeds 90 days (Helius pagination constraint)

---

### `PoolFilter`

```
PoolFilter {
    protocol:      Protocol,
    specific_pool: Option<String>,
}

Protocol {
    Raydium,
    Orca,
    Both,
}
```

Optional filters applied during position discovery. When
`specific_pool` is `None`, all pools for the wallet are analyzed.

---

### `Config`

```
Config {
    wallet:         WalletAddress,
    date_range:     DateRange,
    filter:         PoolFilter,
    rpc_url:        String,
    helius_api_key: String,
}
```

The single source of truth for everything the user provided.
Constructed once at startup. Passed read-only to every layer.

**Security note:** `helius_api_key` is never logged, never cached to
disk, and never included in any error message or debug output.

---

## Layer 2 — Fetcher types

### `PersonalPositionState`

Deserialized from a Raydium CLMM PersonalPosition on-chain account.
Represents one LP position opened by the wallet.

**Key fields:**

| Field | Type | Unit | Meaning |
|-------|------|------|---------|
| `pool_id` | Pubkey | — | Which pool this position is in |
| `tick_lower_index` | i32 | tick | Lower price bound |
| `tick_upper_index` | i32 | tick | Upper price bound |
| `liquidity` | u128 | raw | Liquidity units deposited |
| `fee_growth_inside_0_last` | u128 | Q128.128 | Fee checkpoint token 0 |
| `fee_growth_inside_1_last` | u128 | Q128.128 | Fee checkpoint token 1 |

**Source:** `getProgramAccounts` filtered by Raydium CLMM program ID
and wallet pubkey. Deserialized via `BorshDeserialize`.

---

### `PoolState`

Deserialized from a Raydium CLMM PoolState on-chain account.
Represents the current state of one liquidity pool.

**Key fields:**

| Field | Type | Unit | Meaning |
|-------|------|------|---------|
| `sqrt_price_x64` | u128 | Q64.64 | Current price in fixed-point format |
| `tick_current` | i32 | tick | Current active tick |
| `fee_rate` | u16 | bps | Pool fee rate |
| `liquidity` | u128 | raw | Total active liquidity |
| `fee_growth_global_0` | u128 | Q128.128 | Accumulated fees per unit liquidity, token 0 |
| `fee_growth_global_1` | u128 | Q128.128 | Accumulated fees per unit liquidity, token 1 |

**Source:** `getMultipleAccounts` for each pool ID found in positions.

---

### `FetchResult`

```
FetchResult {
    positions:            Vec<PersonalPositionState>,
    pool_states:          HashMap<Pubkey, PoolState>,
    transactions_by_pool: HashMap<Pubkey, Vec<EncodedTransaction>>,
    slot_start:           u64,
    slot_end:             u64,
}
```

The complete output of the fetching layer. Everything the parsing layer
needs, nothing more. The slot bounds are the converted form of the
user's date range.

---

## Layer 3 — Parser types

### `SwapEvent`

The central type of the entire system. Every downstream computation
operates on a list of SwapEvents.

```
SwapEvent {
    slot:              u64,
    timestamp:         i64,
    pool:              Pubkey,
    sqrt_price_before: u128,
    sqrt_price_after:  u128,
    price_before:      f64,
    price_after:       f64,
    active_liquidity:  u128,
    fee_rate:          u16,
    direction:         SwapDirection,
}
```

**Why both raw and float prices:** The raw Q64.64 values are the
mathematically correct inputs to the LVR formula — squaring an already
rounded float amplifies rounding error quadratically. The float values
are for display and the volatility estimator. See ADR-0005.

**Invariants:**
- `slot` is always within the analysis date range slot bounds
- `price_before` and `price_after` are always positive
- `active_liquidity` is always positive
- A `Vec<SwapEvent>` is always sorted by `slot` ascending
- No two events in a list share the same slot and pool combination

---

### `SwapDirection`

```
SwapDirection {
    ZeroForOne,   // token0 in, token1 out — price decreases
    OneForZero,   // token1 in, token0 out — price increases
}
```

---

## Layer 4 — Engine types

### `LvrResult`

```
LvrResult {
    total_lvr_usd:      f64,
    event_count:        usize,
    largest_single_lvr: f64,
    lvr_by_day:         Vec<(NaiveDate, f64)>,
}
```

`lvr_by_day` is the daily breakdown used by the regime classifier to
identify which market conditions drove the most extraction.

---

### `FeeResult`

```
FeeResult {
    fees_token_0: u64,
    fees_token_1: u64,
    fees_usd:     f64,
}
```

`fees_token_0` and `fees_token_1` are in the token's native decimal
units. `fees_usd` converts both to USD at the price at the end of the
analysis period.

---

### `RegimeResult`

```
RegimeResult {
    annualized_volatility: f64,
    regime:                Regime,
    trending_fraction:     f64,
}

Regime {
    Trending,      // trending_fraction > 0.7
    Volatile,      // trending_fraction < 0.3 and volatility > 0.6
    MeanReverting, // everything else
}
```

`annualized_volatility` is a decimal fraction — 0.80 means 80%
annualized volatility. `trending_fraction` is net price movement
divided by total absolute price movement, ranging from 0.0 to 1.0.

---

### `Verdict`

```
Verdict {
    ratio:       f64,
    label:       VerdictLabel,
    net_pnl_usd: f64,
}

VerdictLabel {
    Profitable,   // ratio > 1.2
    Marginal,     // ratio 0.9 to 1.2
    Unprofitable, // ratio < 0.9
    Inactive,     // range_efficiency < 0.2
}
```

`ratio` is `fees_usd / total_lvr_usd`. A ratio above 1.0 means the
position earned more in fees than it paid in LVR — it was profitable
on a risk-adjusted basis.

---

### `OptimizerResult`

```
OptimizerResult {
    optimal_params:  ParameterSet,
    projected_ratio: f64,
    confidence:      ConfidenceLevel,
    runner_up:       Option<ParameterSet>,
}

ParameterSet {
    fee_tier:    FeeTier,
    range_width: f64,      // symmetric range as decimal — 0.08 means ±8%
}

FeeTier {
    OneBps,      //  0.01%
    FiveBps,     //  0.05%
    TwentyFiveBps, // 0.25%
    HundredBps,  //  1.00%
}

ConfidenceLevel {
    High,   // projected ratio > 1.3 and stable single regime
    Medium, // projected ratio 1.1 to 1.3
    Low,    // projected ratio < 1.1 or unstable regime
}
```

`runner_up` is the second-best parameter combination. When it is close
to `optimal_params`, the recommendation plateau is wide and the
suggestion is more robust to regime changes.

---

### `PositionAnalysis`

The complete output of the engine layer. The output layer consumes this
type and nothing else.

```
PositionAnalysis {
    position:         PersonalPositionState,
    lvr:              LvrResult,
    fees:             FeeResult,
    verdict:          Verdict,
    range_efficiency: f64,
    regime:           RegimeResult,
    optimizer:        OptimizerResult,
    event_count:      usize,
    analysis_period:  DateRange,
}
```

---

## Type state diagram

How raw data flows through the system and becomes a `PositionAnalysis`:

```
CLI args
    │
    │  Config::from_env_and_args()
    ▼
Config
    │
    │  FetchPipeline::run()
    ▼
FetchResult
(PersonalPositionState + PoolState + raw transactions)
    │
    │  parse_pool_transactions()
    ▼
Vec<SwapEvent>
(sorted by slot, filtered to tick range, deduplicated)
    │
    │  PositionAnalysis::compute()
    ▼
PositionAnalysis
(LvrResult + FeeResult + Verdict + RegimeResult + OptimizerResult)
    │
    │  print_output()
    ▼
stdout
```

Each arrow is a named function with a clear signature. Each box is a
concrete Rust type. There are no implicit transformations anywhere in
this pipeline.