# Contributing

This document describes how to set up the development environment,
run the tests, and follow the standards used in this project.

---

## Development environment setup

### Requirements

- Rust stable toolchain (install via rustup.rs)
- Git
- A Helius API key (free tier at helius.dev)

### Install the Rust toolchain

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup component add rustfmt clippy
```

### Clone the repository

```bash
git clone git@github.com:sabijuraa/lvr-meter.git
cd lvr-meter
```

### Set required environment variables

On Linux or Git Bash:

```bash
export HELIUS_API_KEY=your_key_here
```

On Windows PowerShell:

```powershell
$env:HELIUS_API_KEY = "your_key_here"
```

### Build and verify

```bash
cargo build
cargo clippy -- -D warnings
cargo test
```

All three must pass before you write a single line of new code.

---

## Running the tests

### Unit tests only (fast, no network)

```bash
cargo test
```

### Integration tests (requires network and API key)

Integration tests are marked `#[ignore]` so they do not run in CI by
default. Run them manually:

```bash
cargo test -- --ignored
```

### End-to-end test against mainnet

```bash
cargo test e2e -- --ignored
```

This runs the full pipeline against a known public wallet on Solana
mainnet. It takes 3-8 minutes on the first run and uses Helius API
credits. Subsequent runs with the same date range return instantly
from cache.

---

## Code standards

### Every commit must

- Compile — `cargo build` passes
- Be clippy clean — `cargo clippy -- -D warnings` passes with zero warnings
- Be formatted — `cargo fmt` has been run
- Pass all tests — `cargo test` passes

No exceptions. CI enforces all four automatically on every push.

---

## Commit message format

This project uses Conventional Commits (conventionalcommits.org).

```
type(scope): short description in present tense

Optional longer description explaining why, not what.
The what is visible in the diff. The why is not.
```

### Types

| Type | When to use |
|------|-------------|
| `feat` | New feature or capability |
| `fix` | Bug fix |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `test` | Adding or fixing tests |
| `docs` | Documentation only |
| `chore` | Build process, dependencies, tooling |

### Scopes

| Scope | What it covers |
|-------|----------------|
| `config` | src/config/ |
| `fetcher` | src/fetcher/ |
| `parser` | src/parser/ |
| `engine` | src/engine/ |
| `output` | src/output/ |
| `cli` | CLI argument parsing |
| `ci` | GitHub Actions workflows |
| `adr` | Architecture Decision Records |
| `deps` | Cargo.toml dependency changes |
| `repo` | Repository structure |

### Examples

```
feat(config): add WalletAddress newtype with base58 validation
fix(parser): correct sqrtPriceX64 decimal adjustment for token pairs
test(engine): add property tests for optimizer parameter constraints
docs(adr): add ADR-0009 async runtime selection
chore(deps): bump reqwest to 0.12.5
```

---

## One logical change per commit

A commit that adds a type, writes tests for it, and fixes a bug in an
unrelated module is three commits — not one. Each commit answers one
question: what changed and why?

If you cannot write the commit message in one line, the commit is
too large.

---

## Doc comments on all public items

Every public function, struct, and enum must have a doc comment.

```rust
/// Validates and wraps a Solana wallet address.
///
/// Ensures the address is valid base58 and decodes to exactly 32 bytes.
/// Use `WalletAddress::parse` to construct — never construct directly.
pub struct WalletAddress(String);
```

---

## Architecture Decision Records

If your change makes an architectural decision — choosing a library,
choosing a data representation, choosing an algorithm — it needs an ADR
written before the implementation begins.

See `docs/adr/README.md` for the full ADR process and
`docs/adr/TEMPLATE.md` for the standard format.

When in doubt, write an ADR. A short ADR for a small decision is better
than no ADR for a large decision.

---

## Adding test fixtures

Real transaction fixtures live in `fixtures/`. These are JSON files
containing actual Solana transactions fetched from mainnet, used as
test inputs for the parser layer.

To add a new fixture:

1. Fetch the transaction using the Solana CLI or Helius API
2. Save the raw JSON response to `fixtures/tx_<description>.json`
3. Redact any wallet addresses you prefer not to make public —
   replace with `WALLET_REDACTED`
4. Add a unit test in `tests/` that deserializes the fixture and
   asserts the expected parsed output

---

## Project structure

```
lvr-meter/
├── src/
│   ├── main.rs          Entry point and pipeline orchestration
│   ├── config/          CLI parsing and input validation
│   ├── fetcher/         RPC calls, account reading, tx history, cache
│   ├── parser/          Transaction decoding and SwapEvent construction
│   ├── engine/          LVR math, fees, volatility, optimizer
│   └── output/          Table rendering and JSON serialization
├── tests/               Integration and end-to-end tests
├── fixtures/            Real transaction JSON files for parser tests
├── docs/
│   ├── adr/             Architecture Decision Records
│   ├── architecture.md  System design and layer boundaries
│   ├── math.md          Mathematical specification
│   ├── data-model.md    Type system and data flow
│   └── contributing.md  This file
├── .github/
│   ├── workflows/       CI and release automation
│   ├── ISSUE_TEMPLATE/  Bug report and feature request templates
│   └── PULL_REQUEST_TEMPLATE.md
├── CHANGELOG.md         Version history
├── SECURITY.md          Security policy
└── README.md            Project overview
```

---

## Getting help

If something in the development setup does not work, open an issue
using the bug report template. Include your OS, Rust version, and
the exact error message.