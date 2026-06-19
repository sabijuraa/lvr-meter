# Transaction Fixtures

Real Raydium CLMM swap transactions saved as JSON for parser regression tests.

## How to fetch fixtures

Install Solana CLI, then run from project root:

```bash
./scripts/fetch_fixtures.sh
```

Or fetch manually:

```bash
RPC="https://mainnet.helius-rpc.com/?api-key=YOUR_KEY"

# Known SOL/USDC Raydium CLMM swap signatures
SIGS=(
  "5NiuFAFimqFxFMtzXJhcYtGkb8AaFn9fFJMt3aRj3xJqcUJmTgEY9nFxFzSHrKv2PqHxPGRSDzRCeJjGMGmQBqx"
  "3vZ8GmExampleSignature2RaydiumSwap"
)

mkdir -p tests/fixtures

for SIG in "${SIGS[@]}"; do
  curl "$RPC" \
    -H "Content-Type: application/json" \
    -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getTransaction\",\"params\":[\"$SIG\",{\"encoding\":\"json\",\"maxSupportedTransactionVersion\":0}]}" \
    | jq '.result' > "tests/fixtures/${SIG}.json"
done
```

## Fixture files

Each file is a single `EncodedTransactionWithStatusMeta` JSON object.