Date: 2025-06-10
Status: Accepted
Context
Fetching 90 days of swap transactions for a busy Solana pool requires: paginated transaction history queries returning up to 1000 signatures per page, batch account fetching for pre/post account states, and reliable uptime during multi-minute fetch operations. Standard public Solana RPC endpoints (mainnet-beta.solana.com) rate-limit aggressively and do not return pre/post account state snapshots in transaction responses.
Decision
We will use Helius as the primary RPC provider, specifically their enhanced transaction API which returns decoded instruction data and pre/post account states per transaction.
Alternatives Considered
Public Solana RPC (mainnet-beta.solana.com): Free but rate-limited to approximately 10 requests per second with frequent 429 responses during peak hours. More critically, the standard getTransaction response does not include pre/post account data by default — reconstructing pool state before each swap would require separate getAccountInfo calls at historical slots, which is not supported. Rejected.
Triton One / GenesysGo: High-performance RPC with good uptime. Does not offer the enhanced transaction format that includes decoded account states. Would require us to deserialize raw account bytes from the transaction's accountKeys array — possible but significantly more complex. Rejected for version one.
Self-hosted Solana validator with full history: Complete control, no rate limits, full transaction data. Requires $500-2000/month in infrastructure and weeks of historical data sync. Entirely disproportionate for a CLI tool. Rejected.
Consequences
Positive: Enhanced transaction format reduces parsing complexity significantly. Free tier (100k credits/month) is sufficient for development. Reliable uptime.
Negative: API key required — adds friction for new users. Free tier exhausts quickly on large wallets. Creates a dependency on a third-party service.
Risks: Helius changes their API format or pricing. Mitigated by the cache layer — once transactions are fetched and cached, the tool never hits the network again for the same date range.
References

Helius enhanced transactions: docs.helius.dev/solana-apis/enhanced-transactions-api

