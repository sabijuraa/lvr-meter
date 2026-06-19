use anyhow::Result;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;

use crate::fetcher::raydium::pool_state::PoolState;
use crate::fetcher::rpc::RpcClientWrapper;

const BATCH_SIZE: usize = 100;

pub fn fetch_pool_states(
    pool_ids: &[Pubkey],
    client: &RpcClientWrapper,
) -> Result<HashMap<Pubkey, PoolState>> {
    let mut result = HashMap::new();

    for chunk in pool_ids.chunks(BATCH_SIZE) {
        let accounts = client.call_with_retry("get_multiple_accounts", || {
            client.client.get_multiple_accounts(chunk)
        })?;

        for (pubkey, maybe_account) in chunk.iter().zip(accounts.iter()) {
            match maybe_account {
                Some(account) => {
                    match PoolState::from_account_data(&account.data) {
                        Ok(state) => { result.insert(*pubkey, state); }
                        Err(e) => tracing::warn!(
                            "Failed to deserialize PoolState for {}: {}",
                            pubkey, e
                        ),
                    }
                }
                None => tracing::warn!("Pool account not found: {}", pubkey),
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fetcher::raydium::pool_state::POOL_STATE_DISCRIMINATOR;

    fn build_pool_account_data(
        sqrt_price_x64: u128,
        tick_current: i32,
        liquidity: u128,
    ) -> Vec<u8> {
        let mut data = vec![0u8; 512];
        data[..8].copy_from_slice(&POOL_STATE_DISCRIMINATOR);
        data[8 + 225] = 9;
        data[8 + 226] = 6;
        data[8 + 227..8 + 229].copy_from_slice(&10u16.to_le_bytes());
        data[8 + 229..8 + 245].copy_from_slice(&liquidity.to_le_bytes());
        data[8 + 245..8 + 261].copy_from_slice(&sqrt_price_x64.to_le_bytes());
        data[8 + 261..8 + 265].copy_from_slice(&tick_current.to_le_bytes());
        data[8 + 269..8 + 285].copy_from_slice(&0u128.to_le_bytes());
        data[8 + 285..8 + 301].copy_from_slice(&0u128.to_le_bytes());
        data
    }

    #[test]
    fn deserializes_pool_state_from_fixture() {
        let sqrt = 18_446_744_073_709_551_616u128;
        let data = build_pool_account_data(sqrt, -100, 5_000_000);
        let state = PoolState::from_account_data(&data).unwrap();

        assert_eq!(state.sqrt_price_x64, sqrt);
        assert_eq!(state.tick_current, -100);
        assert_eq!(state.liquidity, 5_000_000);
        assert_eq!(state.mint_decimals_0, 9);
        assert_eq!(state.mint_decimals_1, 6);
        assert_eq!(state.tick_spacing, 10);
    }

    #[test]
    fn chunks_large_input() {
        // Verify chunks(100) splits correctly — no RPC needed
        let pool_ids: Vec<Pubkey> = (0..250)
            .map(|_| Pubkey::new_unique())
            .collect();

        let chunks: Vec<&[Pubkey]> = pool_ids.chunks(BATCH_SIZE).collect();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 100);
        assert_eq!(chunks[1].len(), 100);
        assert_eq!(chunks[2].len(), 50);
    }
}