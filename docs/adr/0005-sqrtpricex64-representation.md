Date: 2025-06-10
Status: Accepted
Context
Raydium CLMM stores price as a Q64.64 fixed-point number: the square root of the price ratio multiplied by 2^64, stored as a u128. For LVR computation we need floating-point prices. For display we need human-readable prices. The conversion from sqrtPriceX64 to float involves squaring a u128 and dividing by 2^128 — an operation that loses precision if done naively in f64.
Decision
The SwapEvent type retains both the raw sqrt_price_x64: u128 values (before and after) and the derived price_before: f64 and price_after: f64. The LVR formula uses the float values. The float conversion is performed once at parse time with explicit decimal adjustment. All display logic uses the float values. No other code touches the raw u128 values after parsing.
Alternatives Considered
Store only float prices: Simpler SwapEvent type. Rejected because float prices introduce accumulated rounding error across millions of events. More importantly, the sqrtPriceX64 difference is the mathematically correct input to Theorem 1 — the paper's formula operates on the squared price difference, and squaring an already-rounded float amplifies the rounding error quadratically.
Store only raw u128 and convert lazily: Defers conversion, avoids premature rounding. Rejected because lazy conversion means the conversion logic is scattered across the engine layer rather than centralized in the parser. A single well-tested conversion function at parse time is safer than multiple conversion call sites.
Use a fixed-point arithmetic crate (fixed or substrate-fixed): Maintains full precision throughout the pipeline. Rejected because the precision gain over f64 is negligible at the price levels and trade sizes relevant to this tool, and fixed-point arithmetic significantly complicates the LVR and volatility computations which are inherently statistical.
Consequences
Positive: Single conversion point, easy to test and audit. Float prices are immediately usable by the engine layer. Raw values available for future high-precision extensions.
Negative: SwapEvent is slightly larger in memory. The u128 fields are unused after parsing in version one.
References

Uniswap v3 whitepaper section 6.1: sqrtPriceX96 representation (Solana uses X64 variant)