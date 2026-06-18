use anyhow::{anyhow, bail, Result};
use solana_sdk::pubkey::Pubkey;

pub const POOL_STATE_DISCRIMINATOR: [u8; 8] =
    [0xf2, 0x47, 0x05, 0xa5, 0x2a, 0x35, 0xa3, 0x6d];

/// Subset of PoolState fields needed for LVR computation.
/// Full on-chain struct uses #[zero_copy] with #[repr(C, packed)] —
/// we deserialize by reading raw byte offsets, not Borsh.
///
/// Byte layout after 8-byte discriminator:
///   bump          [u8;1]   offset   0
///   amm_config    Pubkey   offset   1
///   owner         Pubkey   offset  33
///   token_mint_0  Pubkey   offset  65
///   token_mint_1  Pubkey   offset  97
///   token_vault_0 Pubkey   offset 129
///   token_vault_1 Pubkey   offset 161
///   obs_key       Pubkey   offset 193
///   mint_dec_0    u8       offset 225
///   mint_dec_1    u8       offset 226
///   tick_spacing  u16      offset 227
///   liquidity     u128     offset 229
///   sqrt_price_x64 u128   offset 245
///   tick_current  i32      offset 261
///   padding3      u16      offset 265
///   padding4      u16      offset 267
///   fee_growth_0  u128     offset 269
///   fee_growth_1  u128     offset 285
#[derive(Clone, Debug)]
pub struct PoolState {
    pub sqrt_price_x64:        u128,
    pub tick_current:          i32,
    pub liquidity:             u128,
    pub fee_growth_global_0_x64: u128,
    pub fee_growth_global_1_x64: u128,
    pub mint_decimals_0:       u8,
    pub mint_decimals_1:       u8,
    pub tick_spacing:          u16,
}

impl PoolState {
    pub fn from_account_data(data: &[u8]) -> Result<Self> {
        if data.len() < 301 {
            bail!("PoolState account data too short: {} bytes", data.len());
        }

        // All offsets are relative to start of data (include 8-byte discriminator)
        let body = &data[8..]; // skip discriminator

        // body offsets (0-indexed after discriminator):
        //   0:   bump       [u8;1]
        //   1:   amm_config [u8;32]
        //  33:   owner      [u8;32]
        //  65:   token_mint_0 [u8;32]
        //  97:   token_mint_1 [u8;32]
        // 129:   token_vault_0 [u8;32]
        // 161:   token_vault_1 [u8;32]
        // 193:   observation_key [u8;32]
        // 225:   mint_decimals_0 u8
        // 226:   mint_decimals_1 u8
        // 227:   tick_spacing u16
        // 229:   liquidity u128
        // 245:   sqrt_price_x64 u128
        // 261:   tick_current i32
        // 265:   padding3 u16
        // 267:   padding4 u16
        // 269:   fee_growth_global_0_x64 u128
        // 285:   fee_growth_global_1_x64 u128

        let mint_decimals_0 = body[225];
        let mint_decimals_1 = body[226];
        let tick_spacing    = read_u16_le(body, 227)?;
        let liquidity       = read_u128_le(body, 229)?;
        let sqrt_price_x64  = read_u128_le(body, 245)?;
        let tick_current    = read_i32_le(body, 261)?;
        let fee_growth_0    = read_u128_le(body, 269)?;
        let fee_growth_1    = read_u128_le(body, 285)?;

        Ok(Self {
            sqrt_price_x64,
            tick_current,
            liquidity,
            fee_growth_global_0_x64: fee_growth_0,
            fee_growth_global_1_x64: fee_growth_1,
            mint_decimals_0,
            mint_decimals_1,
            tick_spacing,
        })
    }
}

fn read_u16_le(data: &[u8], offset: usize) -> Result<u16> {
    data.get(offset..offset + 2)
        .ok_or_else(|| anyhow!("read_u16: out of bounds at offset {}", offset))
        .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
}

fn read_i32_le(data: &[u8], offset: usize) -> Result<i32> {
    data.get(offset..offset + 4)
        .ok_or_else(|| anyhow!("read_i32: out of bounds at offset {}", offset))
        .map(|b| i32::from_le_bytes(b.try_into().unwrap()))
}

fn read_u128_le(data: &[u8], offset: usize) -> Result<u128> {
    data.get(offset..offset + 16)
        .ok_or_else(|| anyhow!("read_u128: out of bounds at offset {}", offset))
        .map(|b| u128::from_le_bytes(b.try_into().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_data(
        sqrt_price_x64: u128,
        tick_current: i32,
        liquidity: u128,
        fee_growth_0: u128,
        fee_growth_1: u128,
    ) -> Vec<u8> {
        let mut data = vec![0u8; 512];

        // discriminator
        data[..8].copy_from_slice(&POOL_STATE_DISCRIMINATOR);

        // body offset 225: mint_decimals_0
        data[8 + 225] = 9;
        // body offset 226: mint_decimals_1
        data[8 + 226] = 6;
        // body offset 227: tick_spacing u16
        data[8 + 227..8 + 229].copy_from_slice(&10u16.to_le_bytes());
        // body offset 229: liquidity u128
        data[8 + 229..8 + 245].copy_from_slice(&liquidity.to_le_bytes());
        // body offset 245: sqrt_price_x64 u128
        data[8 + 245..8 + 261].copy_from_slice(&sqrt_price_x64.to_le_bytes());
        // body offset 261: tick_current i32
        data[8 + 261..8 + 265].copy_from_slice(&tick_current.to_le_bytes());
        // body offset 269: fee_growth_global_0_x64 u128
        data[8 + 269..8 + 285].copy_from_slice(&fee_growth_0.to_le_bytes());
        // body offset 285: fee_growth_global_1_x64 u128
        data[8 + 285..8 + 301].copy_from_slice(&fee_growth_1.to_le_bytes());

        data
    }

    #[test]
    fn deserializes_sqrt_price() {
        let sqrt = 18_446_744_073_709_551_616u128; // 2^64 = price of 1.0
        let pool = PoolState::from_account_data(
            &build_test_data(sqrt, 0, 1_000_000, 0, 0)
        ).unwrap();
        assert_eq!(pool.sqrt_price_x64, sqrt);
    }

    #[test]
    fn deserializes_tick_current() {
        let pool = PoolState::from_account_data(
            &build_test_data(1, -500, 0, 0, 0)
        ).unwrap();
        assert_eq!(pool.tick_current, -500);
    }

    #[test]
    fn deserializes_fee_growth_globals() {
        let pool = PoolState::from_account_data(
            &build_test_data(1, 0, 0, 123_456_789, 987_654_321)
        ).unwrap();
        assert_eq!(pool.fee_growth_global_0_x64, 123_456_789);
        assert_eq!(pool.fee_growth_global_1_x64, 987_654_321);
    }

    #[test]
    fn deserializes_decimals_and_tick_spacing() {
        let pool = PoolState::from_account_data(
            &build_test_data(1, 0, 1_000_000, 0, 0)
        ).unwrap();
        assert_eq!(pool.mint_decimals_0, 9);
        assert_eq!(pool.mint_decimals_1, 6);
        assert_eq!(pool.tick_spacing, 10);
    }

    #[test]
    fn rejects_too_short() {
        assert!(PoolState::from_account_data(&[0u8; 50]).is_err());
    }
}