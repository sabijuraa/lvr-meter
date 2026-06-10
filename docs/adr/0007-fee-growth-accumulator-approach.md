Date: 2025-06-10
Status: Accepted
Context
Computing fees earned by an LP position can be approached in two ways: summing the fee amount emitted in each swap transaction that touched the position, or using the fee growth accumulator mechanism built into Raydium's PoolState. Both should produce the same result. The question is which approach is more reliable and simpler to implement correctly.
Decision
We will use the fee growth accumulator approach: read fee_growth_global_0 and fee_growth_global_1 from PoolState at position open and close, subtract the position's fee_growth_inside_0_last and fee_growth_inside_1_last checkpoints, and multiply by the position's liquidity divided by 2^128. This is exactly the computation Raydium's on-chain program performs when a user claims fees.
Alternatives Considered
Transaction-level fee parsing: Parse each swap transaction and extract the fee allocated to the position from the instruction logs or account deltas. Theoretically gives a per-swap fee breakdown. Rejected because Raydium does not emit per-LP fee amounts in individual swap logs — the fee is accumulated globally and only distributed when explicitly claimed. Attempting to reconstruct per-LP fees from swap events would require tracking the position's liquidity share relative to total liquidity on every single swap, which requires additional RPC calls and introduces significant complexity.
Using the token balance delta of the position's fee token accounts: Read the position's associated token accounts before and after a claim transaction to measure actual fees received. Accurate for claimed fees but misses uncollected fees. Also requires the user to have claimed fees during the analysis period, which is not guaranteed. Rejected.
Consequences
Positive: Single computation using two PoolState snapshots and the position's checkpoint values — already in the data model. This is the canonical fee computation used by Raydium's own program. No additional RPC calls needed.
Negative: Requires accurate PoolState snapshots at position open and close timestamps. Snapshot accuracy depends on transaction history completeness — if we miss the transaction that first opened the position, the open-snapshot fee growth values will be slightly off.
Risks: Tick-range fee growth (as distinct from global fee growth) requires checking whether the current tick is above, below, or inside the position's range and applying the appropriate fee growth accounting. Incorrect handling of this edge case produces silent wrong results. Mitigated by the fixture test in commit 37 that verifies against Raydium's UI.
References

Uniswap v3 whitepaper section 6.3: fee growth inside a tick range
Raydium CLMM fee collection: github.com/raydium-io/raydium-clmm/blob/master/programs/amm/src/instructions/collect_remaining_rewards.rs

