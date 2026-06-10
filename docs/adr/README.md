# Architecture Decision Records

This directory contains the Architecture Decision Records (ADRs) for lvr-meter.

An ADR documents one architectural decision: what was decided, why it was
decided, what alternatives were rejected, and what the consequences are.

ADRs are never deleted or modified after merging. If a decision changes,
a new ADR is written that supersedes the old one. This gives the project
a permanent, honest record of how thinking evolved over time.

---

## What is an ADR and why does this project use them

When a hiring manager or protocol engineer reads this repository, the code
tells them what the tool does. The ADRs tell them how the author thinks.

Every non-trivial architectural choice — which RPC provider to use, how to
represent prices, whether to use a database or flat files, which algorithm
to use for optimization — involves trade-offs. The ADR documents those
trade-offs explicitly. A future contributor reading the code six months from
now will understand not just what was built but why it was built that way,
and critically, what alternatives were considered and rejected.

---

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0001](0001-use-rust-for-cli.md) | Use Rust for the CLI implementation | Accepted |
| [0002](0002-helius-as-rpc-provider.md) | Use Helius as the RPC provider | Accepted |
| [0003](0003-borsh-deserialization-strategy.md) | Borsh deserialization over manual byte parsing | Accepted |
| [0004](0004-disk-cache-for-transactions.md) | Local disk cache for transaction history | Accepted |
| [0005](0005-sqrtpricex64-representation.md) | Retain raw sqrtPriceX64 alongside float price | Accepted |
| [0006](0006-discrete-time-lvr-approximation.md) | Discrete-time LVR approximation over continuous-time formula | Accepted |
| [0007](0007-fee-growth-accumulator-approach.md) | Fee growth accumulator over transaction-level fee parsing | Accepted |
| [0008](0008-grid-search-over-gradient-descent.md) | Grid search over gradient descent for the optimizer | Accepted |

---

## ADR status definitions

| Status | Meaning |
|--------|---------|
| Proposed | Under discussion, not yet implemented |
| Accepted | Decision made, implementation follows or is complete |
| Deprecated | Was accepted, no longer applies |
| Superseded | Replaced by a newer ADR, linked in the record |

---

## How to write a new ADR

1. Copy `TEMPLATE.md` to a new file named `XXXX-short-title.md`
   where XXXX is the next number in sequence
2. Fill in every section — do not leave placeholders
3. Set status to Proposed
4. Open a PR — the ADR is reviewed before the implementation begins
5. After the PR merges, status changes to Accepted
6. Update this index table

---

## Template

See [TEMPLATE.md](TEMPLATE.md) for the standard ADR format.