# Security Policy

## What this tool does and does not do

lvr-meter is a **read-only analysis tool**. It never submits transactions
to any blockchain. It never modifies any on-chain state. It connects to
Solana RPC endpoints only to read historical data.

Understanding this is important for assessing the security surface:

- No private keys are ever required, stored, or handled
- No transactions are ever signed or broadcast
- The tool cannot move funds under any circumstances

---

## Sensitive data handled by this tool

### Helius API key

The only sensitive credential this tool uses is a Helius RPC API key.

**How it is handled:**
- Read from the `HELIUS_API_KEY` environment variable at runtime
- Never written to disk in any form
- Never included in cache files
- Never logged — not even at debug level
- Never appears in error messages or stack traces

**What to do if your key is exposed:**
1. Go to helius.dev and rotate your API key immediately
2. The old key becomes invalid within seconds of rotation
3. Set your new key in your environment: `export HELIUS_API_KEY=new_key`

### Wallet addresses

Wallet addresses passed via `--wallet` are public by definition on
Solana — they are not secrets. However:

- Wallet addresses are never sent to any service other than the
  configured RPC endpoint
- They are not logged to any external service
- Cache files are stored locally and contain only transaction data
  that is already publicly visible on-chain

### Cache files

The `.lvr-cache/` directory stores raw transaction data fetched from
the blockchain. This data is entirely public — anyone can fetch the
same data from any Solana RPC. The cache contains no credentials,
no private keys, and no data that is not already public.

The cache directory is excluded from version control via `.gitignore`.
Do not commit it manually.

---

## Supported versions

| Version | Supported |
|---------|-----------|
| main branch | Yes |
| Tagged releases | Yes |
| Older releases | No — please upgrade |

---

## Reporting a vulnerability

If you find a security vulnerability in lvr-meter — for example,
a way the tool could be tricked into leaking the API key, a
dependency with a known CVE, or unexpected network behavior —
please report it privately rather than opening a public issue.

**How to report:**

Open a GitHub Security Advisory on this repository:
1. Go to the repository on GitHub
2. Click the Security tab
3. Click "Report a vulnerability"
4. Describe the issue in detail

You will receive a response within 72 hours.

**Please include:**
- A description of the vulnerability
- Steps to reproduce it
- What information could be exposed or what harm could result
- Your assessment of severity

**Please do not:**
- Open a public GitHub issue for security vulnerabilities
- Post details publicly before a fix is available

---

## Dependencies

lvr-meter uses the following external crates. Known vulnerabilities
in dependencies can be checked with:

```bash
cargo install cargo-audit
cargo audit
```

Run `cargo audit` before every release. The CI pipeline does not
currently run cargo-audit automatically — this is a known gap
tracked for the next release.

---

## Threat model

The primary security concern for a tool like lvr-meter is credential
leakage — specifically the Helius API key. The design deliberately
minimizes this risk by reading the key from environment only, never
persisting it, and never including it in any logged or cached output.

A secondary concern is supply chain risk from dependencies. All
dependencies are pinned to exact versions in Cargo.lock. Any
dependency update requires an explicit version bump and a new
Cargo.lock commit, making supply chain changes auditable in git
history.