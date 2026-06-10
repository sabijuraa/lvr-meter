# Mathematical Specification

This document specifies the mathematical foundations of lvr-meter.
Every formula implemented in `src/engine/` is derived from and
cross-referenced to this document.

---

## 1. Loss-Versus-Rebalancing (LVR)

### Background

Every swap against a CLMM pool is executed by an arbitrageur closing
the gap between the pool's stale price and the real-world price on
centralized exchanges. The pool's liquidity providers are the
counterparty to every one of these trades. The value extracted from
LPs in aggregate is called Loss-Versus-Rebalancing.

The name comes from the comparison: LVR measures the difference between
what a continuously rebalancing portfolio would be worth versus what the
LP position is worth, accumulated over the position's lifetime.

---

### Continuous-time formula (Milionis et al. 2022, Theorem 1)

In continuous time under a geometric Brownian motion price process:

```
dLVR = (σ² / 8) × L × dt
```

Where:

- `σ` is instantaneous realized volatility
- `L` is active liquidity in USD
- `dt` is the time increment

---

### Discrete-time approximation (implemented in this tool)

Because Solana provides price data only at discrete swap events, we use
the realized-variance estimator applied per swap:

```
LVR_i = (P_after_i - P_before_i)² / (8 × fee_rate) × L_i
```

Where:

- `P_after_i` is the pool price after swap i
- `P_before_i` is the pool price before swap i
- `fee_rate` is the pool's fee rate as a decimal — 0.0025 for 0.25%
- `L_i` is the active liquidity in USD at the time of swap i

Total LVR for the position:

```
LVR_total = Σ LVR_i    for all swaps i in the position's tick range
```

Implemented in: `src/engine/lvr.rs`

---

### Why fee_rate appears in the denominator

Higher fee pools suppress arbitrage — it is less profitable for an
arbitrageur to trade against a 1% fee pool than a 0.01% fee pool. The
fee rate in the denominator captures this suppression effect. A pool
with a higher fee rate experiences less LVR per unit of price movement
because fewer arbitrage trades are profitable at the margin.

---

### Discrete approximation error

The discrete approximation converges to the continuous formula as swap
frequency increases. For busy pools (SOL/USDC, BTC/USDC) with hundreds
of swaps per hour the approximation error is negligible.

For low-frequency pools with fewer than 10 swaps per day, large price
moves that occur between swaps are not captured. The tool emits a
warning when event count is below 100 for the analysis period.

See ADR-0006 for the decision rationale behind using the discrete
approximation over the continuous formula.

---

## 2. Fee Growth Accumulator

### Background

Raydium CLMM uses a fee growth accumulator mechanism to track fees owed
to each LP position without updating every position on every swap, which
would be prohibitively expensive on-chain.

---

### The accumulator mechanism

The pool maintains two global counters:

```
fee_growth_global_0    accumulated fees per unit of liquidity, token 0
fee_growth_global_1    accumulated fees per unit of liquidity, token 1
```

These counters only ever increase. On every swap, the fee collected is
added to the appropriate counter divided by total active liquidity:

```
fee_growth_global += swap_fee / total_liquidity
```

Each personal position stores a checkpoint of these counters at the
last time fees were collected or the position was modified:

```
fee_growth_inside_0_last
fee_growth_inside_1_last
```

---

### Fee computation

Fees earned since the last checkpoint:

```
fees_owed_0 = (fee_growth_global_0 - fee_growth_inside_0_last)
              × position_liquidity / 2^128

fees_owed_1 = (fee_growth_global_1 - fee_growth_inside_1_last)
              × position_liquidity / 2^128
```

Division by 2^128 comes from the Q128.128 fixed-point representation
used for fee growth values in the Raydium CLMM implementation.

Implemented in: `src/engine/fees.rs`

---

### Tick range correction

Fee growth only accrues when the pool's current tick is within the
position's tick range. When the price is outside the range, no fees
accrue and no LVR is paid. The fee growth inside a tick range requires
checking the relationship between the current tick and the position's
bounds and applying the appropriate correction from the global
accumulators and the per-tick fee growth values.

This correction follows the specification in the Uniswap v3 whitepaper
section 6.3, which Raydium CLMM implements directly.

---

## 3. sqrtPriceX64 Conversion

### Representation

Raydium CLMM stores price as a Q64.64 fixed-point number: the square
root of the price ratio multiplied by 2^64, stored as a u128.

### Conversion to float price

```
price = (sqrt_price_x64 / 2^64)² × (10^decimals_1 / 10^decimals_0)
```

Step by step:

1. Cast `sqrt_price_x64` to f64
2. Divide by 2^64 to get the square root of the raw price ratio
3. Square the result to get the raw price ratio
4. Multiply by the decimal adjustment: `10^decimals_1 / 10^decimals_0`

For SOL/USDC where SOL has 9 decimals and USDC has 6 decimals, the
decimal adjustment is `10^6 / 10^9 = 0.001`.

Implemented in: `src/parser/price.rs`

See ADR-0005 for why both the raw u128 and the float price are retained
in the SwapEvent type.

---

## 4. Realized Volatility

### Log-return computation

For each consecutive pair of swap events i and i+1:

```
r_i = ln(P_after_i / P_before_i)
```

Log-returns are used instead of simple returns because log-returns are
additive over time and symmetric around zero, which makes the standard
deviation estimator unbiased under the log-normal price assumption.

### Annualized volatility

```
σ_annualized = std(r_i) × sqrt(N_annual)
```

Where `N_annual` is the estimated number of swap events per year,
computed from the observed event frequency in the analysis period:

```
N_annual = event_count / analysis_days × 365
```

Implemented in: `src/engine/regime.rs`

---

## 5. Regime Classification

### Trending fraction

```
trending_fraction = |P_last - P_first| / Σ|P_after_i - P_before_i|
```

The numerator is net price movement from start to end of the period.
The denominator is total absolute price movement — the sum of all
individual swap moves. The fraction ranges from 0.0 to 1.0.

A trending_fraction near 1.0 means all movement was in one direction.
A trending_fraction near 0.0 means price oscillated without net
movement.

### Regime labels

```
if trending_fraction > 0.7:
    regime = Trending

elif trending_fraction < 0.3 and σ_annualized > 0.6:
    regime = Volatile

else:
    regime = MeanReverting
```

Implemented in: `src/engine/regime.rs`

---

## 6. Range Efficiency

### Definition

```
range_efficiency = slots_in_range / total_slots_in_analysis_window
```

### Approximation from swap events

Since we have price observations only at swap events and not at every
slot, we approximate using slot gaps between events:

For each consecutive pair of events i and i+1:

- If the price at event i was within the position's price range,
  add `(slot_{i+1} - slot_i)` to `slots_in_range`
- Otherwise do not add

Price bounds from tick indices:

```
P_lower = 1.0001 ^ tick_lower_index
P_upper = 1.0001 ^ tick_upper_index
```

Implemented in: `src/engine/range_efficiency.rs`

---

## 7. Position Optimizer

### Objective

Find the fee tier `f` and range width `w` that maximize the projected
fee-to-LVR ratio given the observed realized volatility `σ`:

```
maximize:   projected_fees(f, w, σ) / projected_LVR(f, w, σ)

subject to:
    f ∈ { 0.0001, 0.0005, 0.0025, 0.01 }
    w ∈ [ 0.01, 0.25 ]
```

### Projected range efficiency

Under a log-normal price process with volatility `σ` over time horizon
`T`, the probability that price stays within a symmetric range of width
`±w` around the current price is approximated by:

```
range_efficiency(w, σ, T) ≈ erf( w / (σ × sqrt(T)) )
```

Where `erf` is the Gauss error function and `T` is normalized to one
year.

### Projected fee income

```
projected_fees = f × daily_volume_estimate × range_efficiency(w, σ, T)
```

The daily volume estimate is derived from the observed swap event
frequency and average trade size in the analysis period.

### Projected LVR

```
projected_LVR = (σ² / 8f) × L × T × range_efficiency(w, σ, T)
```

### Search method

Exhaustive grid search over all parameter combinations. With 4 fee
tiers and 15 range width steps the search space has 60 combinations
and evaluates in under 1ms. See ADR-0008 for the decision rationale
behind grid search over gradient-based methods.

Implemented in: `src/engine/optimizer/`

---

## References

- Milionis, Moallemi, Roughgarden, Zhang — Automated Market Making and
  Loss-Versus-Rebalancing (2022). Primary source for sections 1 and 7.
  arxiv.org/abs/2208.06046

- Cartea, Drissi, Monga — Decentralised Finance and Automated Market
  Making: Predictable Loss and Optimal Liquidity Provisioning (2022).
  Basis for the optimal range framework in section 7.
  arxiv.org/abs/2309.08431

- Adams et al. — Uniswap v3 Core (2021). Section 6.3: fee growth inside
  a tick range. Basis for section 2.
  uniswap.org/whitepaper-v3.pdf

- Angeris, Chitra — Improved Price Oracles: Constant Function Market
  Makers (2020). Mathematical background on AMM price discovery.
  arxiv.org/abs/2003.10001