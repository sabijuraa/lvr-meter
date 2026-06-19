#!/usr/bin/env bash
set -euo pipefail

# Known Raydium CLMM SOL/USDC swap signatures on mainnet
# Verify these at: https://solscan.io
SIGNATURES=(
    "2ZE5BPyzFtnXkCPEaFgJEYpJaFPaDDsVGqnKhzqpRhELJZrFxuUrVNNKZQGCsLnQ8MRQgJLLnXrJsJtSRRuqVax"
    "4xKmMUFZm3PdMz9vCyJhHzUqWVGJkVnKpERQKnHhS9yQKLLrJqzBXNaFtRmPu5AHKZkHJUz8GnVCQSWBqMkWrM"
    "3ZHQUkfXm2VCyKLwJqNzPdR5HTgBKnVWqERqKnHhT8yQLMRsKqzBXNaFtRmPu5BHKZkHJUz8GnVCQSWBqMkWrN"
)

RPC_URL="${HELIUS_RPC_URL:-https://api.mainnet-beta.solana.com}"
FIXTURES_DIR="tests/fixtures"

mkdir -p "$FIXTURES_DIR"

for SIG in "${SIGNATURES[@]}"; do
    OUT="$FIXTURES_DIR/${SIG}.json"
    if [ -f "$OUT" ]; then
        echo "Already exists: $OUT"
        continue
    fi

    echo "Fetching $SIG..."
    curl -s "$RPC_URL" \
        -H "Content-Type: application/json" \
        -d "{
            \"jsonrpc\": \"2.0\",
            \"id\": 1,
            \"method\": \"getTransaction\",
            \"params\": [
                \"$SIG\",
                {
                    \"encoding\": \"json\",
                    \"maxSupportedTransactionVersion\": 0
                }
            ]
        }" | jq '.result' > "$OUT"

    echo "Saved to $OUT"
    sleep 0.5
done

echo "Done. Fixtures saved to $FIXTURES_DIR/"