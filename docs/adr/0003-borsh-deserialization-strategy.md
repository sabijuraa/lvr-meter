Date: 2025-06-10
Status: Accepted
Context
Raydium CLMM's on-chain account data is binary-encoded using Borsh (Binary Object Representation Serializer for Hashing). Each account type has a fixed binary layout. We need to deserialize PersonalPositionState and PoolState accounts from the raw bytes returned by getAccountInfo and getProgramAccounts RPC calls.
Decision
We will use the borsh crate with derived BorshDeserialize implementations, with struct field ordering and types matching Raydium's source definitions exactly.
Alternatives Considered
Manual byte parsing with byteorder: Read each field as a fixed-offset slice and interpret with byteorder crate. Completely explicit but requires maintaining byte offsets manually. Any upstream change to the struct layout silently breaks all offsets with no compile-time error. Rejected.
Anchor account discriminator + bytemuck: Use bytemuck for zero-copy deserialization after checking the 8-byte Anchor discriminator. Faster than Borsh for large accounts but requires repr(C) alignment guarantees that Raydium's structs may not provide. Rejected for version one — correctness over performance at this scale.
Using Raydium's TypeScript SDK and spawning a node process: Technically possible, produces correct results, but introduces a Node.js runtime dependency into a Rust CLI. Rejected.
Consequences
Positive: Struct definition is the single source of truth — field order matches Raydium's Rust source directly. Compile-time type checking on all deserialized fields. Borsh is stable and well-maintained in the Solana ecosystem.
Negative: Must stay in sync with Raydium's struct definitions manually — no automated schema tracking.
Risks: Raydium upgrades their program and changes the PersonalPosition layout without a version bump. Mitigated by the fixture deserialization tests in commits 14 and 15 — a layout change will cause test failures immediately.
References

Raydium CLMM PersonalPositionState: github.com/raydium-io/raydium-clmm/blob/master/programs/amm/src/states/personal_position.rs

