Date: 2025-06-10
Status: Accepted
Context
Fetching 90 days of swap transactions for a busy pool like SOL/USDC on Raydium takes 3-8 minutes and consumes a significant portion of the Helius free tier credit allocation. Historical blockchain data is immutable — a swap that occurred in March will have identical on-chain data today as it did in March. Repeated fetches of the same data for development and testing are wasteful.
Decision
We will cache all fetched transaction data on disk as JSON files in a .lvr-cache/ directory, keyed by pool address and slot range. Cache entries older than 24 hours for data within the last 7 days are invalidated. Cache entries for data older than 7 days never expire.
Alternatives Considered
No cache, always fetch fresh: Simple, always correct, no disk usage. Rejected because development iteration on the parsing and engine layers requires repeatedly re-running the pipeline. Without cache, a 5-minute fetch penalty on every iteration makes development of the math layer impractical.
SQLite cache with indexed queries: More structured than flat JSON files, supports partial queries. Rejected because the cache key (pool + slot range) maps cleanly to a flat file — there is no query pattern that benefits from a database. Adding SQLite adds a dependency and complexity with no practical benefit at this scale.
In-memory cache within a single process run: Eliminates re-fetching within one run but provides no benefit across runs. Rejected because the most common pattern is multiple runs with different analysis parameters on the same wallet and date range.
Redis or external cache: Absurd for a local CLI tool. Rejected immediately.
Consequences
Positive: Development iteration is fast after the first fetch. Helius API credits are conserved. Reproducible results across runs for the same date range.
Negative: Disk usage — a 90-day analysis of a busy pool can cache 50-200MB of transaction JSON. Cache directory must be excluded from version control.
Risks: Cache corruption from interrupted writes. Mitigated by writing to a .tmp file and atomically renaming on completion.
