use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedTransaction;
use std::fs;
use std::path::{Path, PathBuf};

pub struct TxCache {
    cache_dir: PathBuf,
}

impl TxCache {
    pub fn new(cache_dir: impl AsRef<Path>) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    pub fn default_dir() -> Result<Self> {
        Self::new(".lvr-cache")
    }

    fn cache_key(&self, pool: &Pubkey, slot_start: u64, slot_end: u64) -> PathBuf {
        self.cache_dir.join(format!(
            "{}_{}_{}.json",
            pool, slot_start, slot_end
        ))
    }

    pub fn get(
        &self,
        pool: &Pubkey,
        slot_start: u64,
        slot_end: u64,
    ) -> Option<Vec<EncodedTransaction>> {
        let path = self.cache_key(pool, slot_start, slot_end);

        let data = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn set(
        &self,
        pool: &Pubkey,
        slot_start: u64,
        slot_end: u64,
        txs: &[EncodedTransaction],
    ) -> Result<()> {
        let path = self.cache_key(pool, slot_start, slot_end);
        let data = serde_json::to_string(txs)?;
        fs::write(&path, data)?;
        Ok(())
    }

    pub fn exists(&self, pool: &Pubkey, slot_start: u64, slot_end: u64) -> bool {
        self.cache_key(pool, slot_start, slot_end).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_transaction_status::EncodedTransaction;
    use tempfile::TempDir;

    fn temp_cache() -> (TxCache, TempDir) {
        let dir  = TempDir::new().unwrap();
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
    fn cache_dir_is_created_if_missing() {
        let dir   = TempDir::new().unwrap();
        let path  = dir.path().join("nested").join("cache");
        let cache = TxCache::new(&path);
        assert!(cache.is_ok());
        assert!(path.exists());
    }
}