Date: 2025-06-10
Status: Accepted
Context
The Milionis et al. 2022 paper derives LVR in continuous time under a Black-Scholes price process. The formula in continuous time is: dLVR = (σ²/8) * L * dt where σ is instantaneous volatility, L is active liquidity, and dt is the time increment. On Solana, price data is available only at discrete swap events, not as a continuous process. We need a discrete approximation.
Decision
We will use the discrete-time approximation from Theorem 1's corollary: instantaneous LVR per swap = (P_after - P_before)² / (8 * fee_rate) * L, accumulated across all swaps. This is the realized-variance estimator applied to the LVR formula — it converges to the continuous-time value as swap frequency increases.
Alternatives Considered
Continuous-time formula with interpolated prices: Interpolate a continuous price path between swap events using a geometric Brownian motion bridge, then integrate the continuous formula. More mathematically rigorous but introduces the assumption that price evolves as GBM between swaps — an assumption that is not empirically validated for on-chain price processes. Rejected for version one — the additional complexity is not justified by a measurable accuracy improvement at typical Raydium swap frequencies (hundreds per hour for major pools).
TWAP-based volatility with the annualized formula: Compute realized volatility from the swap event stream, then apply the annualized LVR formula (σ²/8 * L * T). Simpler to implement but produces a single aggregate number rather than a per-swap breakdown. This approach cannot produce the daily LVR breakdown needed for the regime classifier. Rejected.
Consequences
Positive: Per-swap LVR is directly computable from the SwapEvent fields already in the data model. The daily breakdown enables the regime classifier. The approximation error is bounded and decreases with swap frequency.
Negative: For very low-frequency pools (fewer than 10 swaps per day), the discrete approximation may underestimate LVR during large price moves that occur between swaps.
Risks: Using this tool on a low-liquidity pool with rare swaps will produce underestimated LVR. Mitigated by adding a warning in the output when the event count is below 100 for the analysis period.
References

Milionis et al. 2022, Theorem 1 and Remark 3 (discrete approximation): arxiv.org/abs/2208.06046

