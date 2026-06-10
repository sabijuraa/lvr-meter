# Changelog

All notable changes to lvr-meter are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Project skeleton with module structure (config, fetcher, parser, engine, output)
- Core dependencies: clap, tokio, serde, reqwest, anyhow, chrono, tracing
- GitHub Actions CI workflow — build, clippy, fmt, test on every push
- GitHub Actions release workflow — compiles and uploads binary on version tags
- PR template and issue templates (bug report, feature request)
- Architecture Decision Records (ADR-0001 through ADR-0008)
- Supporting documentation: architecture, math specification, data model, contributing guide
- SECURITY policy
- CHANGELOG

---

## [0.1.0] — Unreleased

This release will include:

### Added
- CLI interface with `--wallet`, `--from`, `--to`, `--protocol`, `--pool`, `--dry-run` flags
- `WalletAddress` newtype with base58 validation
- `DateRange` type with chrono parsing — max 90 day range enforced
- Solana account reading — PersonalPosition and PoolState deserialization
- Transaction fetching via Helius RPC with pagination and rate limiting
- Local disk cache for transaction history — instant reruns
- LVR computation per swap using Theorem 1 from Milionis et al. 2022
- Fee reconciler using Raydium fee growth accumulator mechanism
- Realized volatility estimator and regime classifier
- Range efficiency calculator
- Position optimizer — grid search over fee tiers and range widths
- Two-section CLI output: historical analysis table + parameter recommendation

### Academic basis
- Milionis, Moallemi, Roughgarden, Zhang — Automated Market Making and
  Loss-Versus-Rebalancing (2022). arxiv.org/abs/2208.06046
- Cartea, Drissi, Monga — Predictable Loss and Optimal Liquidity
  Provisioning (2022). arxiv.org/abs/2309.08431

---

## Version History

| Version | Date | Description |
|---------|------|-------------|
| 0.1.0 | TBD | Initial release |

---

[Unreleased]: https://github.com/sabijuraa/lvr-meter/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/sabijuraa/lvr-meter/releases/tag/v0.1.0