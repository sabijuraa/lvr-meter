## What this PR does

<!-- One paragraph. What problem does this solve? What feature does it add?
Be specific — "adds WalletAddress newtype with base58 validation" not "adds config stuff" -->

## How to test it

<!-- Exact commands a reviewer should run to verify this works.
Example:
  cargo test config::types
  cargo run -- --wallet <addr> --from 2025-01-01 --to 2025-03-31 --dry-run
-->

## Checklist

- [ ] `cargo build` passes
- [ ] `cargo clippy -- -D warnings` passes with zero warnings
- [ ] `cargo fmt` has been run
- [ ] `cargo test` passes
- [ ] New public functions have doc comments
- [ ] New types have unit tests

## Does this require a new ADR?

<!-- If this PR makes an architectural decision — choosing a library,
choosing a data representation, choosing an algorithm — it needs an ADR.
Link it here or explain why one is not needed. -->

- [ ] Yes — ADR-XXXX is included in this PR
- [ ] No — this is an implementation of an already-documented decision

## Does this update the CHANGELOG?

- [ ] Yes — added under [Unreleased]
- [ ] No — this is a chore or docs change that does not affect users

## Related commits / issues

<!-- Link any related issues or the specific commit from the development
plan this PR implements. Example: "Implements commits 3-5 from the
Phase 1 development plan." -->