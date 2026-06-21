use anyhow::Result;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedTransaction;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CACHE_TTL_SECS: u64 = 24 * 60 * 60; // 24 hours

#[derive(Serialize, Deserialize)]
struct CacheEntry {
    written_at: u64,
    txs:        Vec<EncodedTransaction>,
}

pub struct TxCache {
    cache_dir: PathBuf,
    no_cache:  bool,
}

impl TxCache {
    pub fn new(cache_dir: impl AsRef<Path>) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir, no_cache: false })
    }

    pub fn default_dir() -> Result<Self> {
        Self::new(".lvr-cache")
    }

    pub fn with_no_cache(mut self) -> Self {
        self.no_cache = true;
        self
    }

    fn cache_key(&self, pool: &Pubkey, slot_start: u64, slot_end: u64) -> PathBuf {
        self.cache_dir.join(format!(
            "{}_{}_{}.json",
            pool, slot_start, slot_end
        ))
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    pub fn get(
        &self,
        pool:       &Pubkey,
        slot_start: u64,
        slot_end:   u64,
    ) -> Option<Vec<EncodedTransaction>> {
        if self.no_cache {
            return None;
        }

        let path = self.cache_key(pool, slot_start, slot_end);
        let data = fs::read_to_string(&path).ok()?;
        let entry: CacheEntry = serde_json::from_str(&data).ok()?;

        // Invalidate entries older than 24 hours
        let age = Self::now_secs().saturating_sub(entry.written_at);
        if age > CACHE_TTL_SECS {
            tracing::info!(
                "Cache entry expired ({} hours old) for pool {} — re-fetching",
                age / 3600,
                pool
            );
            let _ = fs::remove_file(&path);
            return None;
        }

        Some(entry.txs)
    }

    pub fn set(
        &self,
        pool:       &Pubkey,
        slot_start: u64,
        slot_end:   u64,
        txs:        &[EncodedTransaction],
    ) -> Result<()> {
        let path  = self.cache_key(pool, slot_start, slot_end);
        let entry = CacheEntry {
            written_at: Self::now_secs(),
            txs:        txs.to_vec(),
        };
        let data = serde_json::to_string(&entry)?;
        fs::write(&path, data)?;
        Ok(())
    }

    pub fn exists(&self, pool: &Pubkey, slot_start: u64, slot_end: u64) -> bool {
        if self.no_cache {
            return false;
        }
        // exists() checks the file AND validates the TTL
        self.get(pool, slot_start, slot_end).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_cache() -> (TxCache, TempDir) {
        let dir   = TempDir::new().unwrap();
        let cache = TxCache::new(dir.path()).unwrap();
        (cache, dir)
    }

    #[test]
    fn write_and_read_back() {
        let (cache, _dir) = temp_cache();
        let pool          = Pubkey::new_unique();
        let txs: Vec<EncodedTransaction> = vec![];

        cache.set(&pool, 100, 200, &txs).unwrap();
        let result = cache.get(&pool, 100, 200);
        assert!(result.is_some());
    }

    #[test]
    fn miss_returns_none() {
        let (cache, _dir) = temp_cache();
        let pool          = Pubkey::new_unique();
        assert!(cache.get(&pool, 100, 200).is_none());
    }

    #[test]
    fn exists_returns_true_after_write() {
        let (cache, _dir) = temp_cache();
        let pool          = Pubkey::new_unique();

        assert!(!cache.exists(&pool, 1, 2));
        cache.set(&pool, 1, 2, &[]).unwrap();
        assert!(cache.exists(&pool, 1, 2));
    }

    #[test]
    fn different_slot_ranges_are_separate_keys() {
        let (cache, _dir) = temp_cache();
        let pool          = Pubkey::new_unique();

        cache.set(&pool, 100, 200, &[]).unwrap();
        assert!( cache.exists(&pool, 100, 200));
        assert!(!cache.exists(&pool, 100, 300));
        assert!(!cache.exists(&pool, 200, 300));
    }

    #[test]
    fn no_cache_flag_always_misses() {
        let (cache, _dir) = temp_cache();
        let cache         = cache.with_no_cache();
        let pool          = Pubkey::new_unique();

        cache.set(&pool, 1, 2, &[]).unwrap();
        assert!(cache.get(&pool, 1, 2).is_none());
        assert!(!cache.exists(&pool, 1, 2));
    }

    #[test]
    fn no_cache_flag_does_not_prevent_writes() {
        let dir   = TempDir::new().unwrap();
        let pool  = Pubkey::new_unique();

        let write_cache = TxCache::new(dir.path()).unwrap().with_no_cache();
        write_cache.set(&pool, 1, 2, &[]).unwrap();

        // A normal cache (no --no-cache) CAN read the file written above
        let read_cache = TxCache::new(dir.path()).unwrap();
        assert!(read_cache.exists(&pool, 1, 2));
    }

    #[test]
    fn cache_dir_is_created_if_missing() {
        let dir   = TempDir::new().unwrap();
        let path  = dir.path().join("nested").join("cache");
        let cache = TxCache::new(&path);
        assert!(cache.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn expired_entry_returns_none() {
        let (cache, _dir) = temp_cache();
        let pool          = Pubkey::new_unique();

        // Write an entry with written_at in the past (25 hours ago)
        let path = cache.cache_key(&pool, 1, 2);
        let old_entry = serde_json::json!({
            "written_at": TxCache::now_secs() - (25 * 3600),
            "txs": []
        });
        fs::write(&path, old_entry.to_string()).unwrap();

        // Should be treated as expired
        assert!(cache.get(&pool, 1, 2).is_none());
        assert!(!path.exists()); // expired file is deleted
    }
}