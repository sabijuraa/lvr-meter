Date: 2025-06-10
Status: Accepted
Context
lvr-meter needs to deserialize Solana on-chain account data, perform floating-point financial math across millions of swap events, and run a grid search optimizer — all in a single binary that a technical LP can install and run locally. The account deserialization schemas are defined in Raydium and Orca's Rust codebases. The Solana SDK is Rust-native. The tool needs to be fast enough that a 90-day analysis completes in under 2 minutes on a laptop.
Decision
We will implement the entire tool in Rust using the 2021 edition.
Alternatives Considered
Python with solana-py: Python is faster to write and has rich data science libraries. Rejected because solana-py's account deserialization support is thin — Raydium's PersonalPosition and PoolState schemas require Borsh deserialization that maps directly to the Rust struct definitions in Raydium's source. Reimplementing these schemas in Python introduces a translation layer that is both error-prone and unmaintained. Additionally, a Python binary cannot be distributed as a single executable.
TypeScript with @solana/web3.js: The Solana ecosystem has strong TypeScript tooling and Raydium publishes an official TypeScript SDK. Rejected because the SDK abstracts away the account layout details we need direct access to, and TypeScript's numeric types (all f64) create precision risks for the u128 fee growth accumulator arithmetic.
Consequences
Positive: Direct use of Raydium's Rust struct definitions via the IDL. Native Borsh deserialization. Single compiled binary for distribution. Full type safety across the entire computation pipeline.
Negative: Longer initial development time than Python. Async error handling in Rust is more verbose. New contributors need Rust familiarity.
Risks: Raydium may change their account schema — if they do, deserialization silently produces wrong values unless fixture tests catch it. Mitigated by fixture tests in commits 14 and 15.
References

Raydium CLMM source: github.com/raydium-io/raydium-clmm
Borsh specification: borsh.io

