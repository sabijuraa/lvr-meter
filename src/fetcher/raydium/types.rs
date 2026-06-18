use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::pubkey::Pubkey;

pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
pub const REWARD_NUM: usize = 3;
pub const PERSONAL_POSITION_DISCRIMINATOR: [u8; 8] =
    [0x68, 0x47, 0x8d, 0x68, 0x4d, 0x5c, 0x45, 0x52];

#[derive(BorshDeserialize, BorshSerialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct PositionRewardInfo {
    pub growth_inside_last_x64: u128,
    pub reward_amount_owed:     u64,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug, Default)]
pub struct PersonalPositionState {
    pub bump:                        [u8; 1],
    pub nft_mint:                    Pubkey,
    pub pool_id:                     Pubkey,
    pub tick_lower_index:            i32,
    pub tick_upper_index:            i32,
    pub liquidity:                   u128,
    pub fee_growth_inside_0_last_x64: u128,
    pub fee_growth_inside_1_last_x64: u128,
    pub token_fees_owed_0:           u64,
    pub token_fees_owed_1:           u64,
    pub reward_infos:                [PositionRewardInfo; REWARD_NUM],
    pub recent_epoch:                u64,
    pub padding:                     [u64; 7],
}

impl PersonalPositionState {
    pub fn from_account_data(data: &[u8]) -> anyhow::Result<Self> {
        if data.len() < 8 {
            anyhow::bail!("Account data too short: {} bytes", data.len());
        }
        Self::try_from_slice(&data[8..])
            .map_err(|e| anyhow::anyhow!("Deserialization failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_data(tick_lower: i32, tick_upper: i32, liquidity: u128) -> Vec<u8> {
        let mut data = Vec::new();

        data.extend_from_slice(&PERSONAL_POSITION_DISCRIMINATOR);
        data.push(255u8);
        data.extend_from_slice(&[1u8; 32]); // nft_mint
        data.extend_from_slice(&[2u8; 32]); // pool_id
        data.extend_from_slice(&tick_lower.to_le_bytes());
        data.extend_from_slice(&tick_upper.to_le_bytes());
        data.extend_from_slice(&liquidity.to_le_bytes());
        data.extend_from_slice(&100u128.to_le_bytes()); // fee_growth_0
        data.extend_from_slice(&200u128.to_le_bytes()); // fee_growth_1
        data.extend_from_slice(&500u64.to_le_bytes());  // fees_owed_0
        data.extend_from_slice(&600u64.to_le_bytes());  // fees_owed_1

        for _ in 0..REWARD_NUM {
            data.extend_from_slice(&0u128.to_le_bytes());
            data.extend_from_slice(&0u64.to_le_bytes());
        }

        data.extend_from_slice(&42u64.to_le_bytes()); // recent_epoch
        for _ in 0..7 {
            data.extend_from_slice(&0u64.to_le_bytes()); // padding
        }

        data
    }

    #[test]
    fn deserializes_tick_range() {
        let pos = PersonalPositionState::from_account_data(
            &build_test_data(-100, 200, 1_000_000)
        ).unwrap();
        assert_eq!(pos.tick_lower_index, -100);
        assert_eq!(pos.tick_upper_index, 200);
    }

    #[test]
    fn deserializes_liquidity() {
        let pos = PersonalPositionState::from_account_data(
            &build_test_data(-100, 200, 999_888_777_666)
        ).unwrap();
        assert_eq!(pos.liquidity, 999_888_777_666);
    }

    #[test]
    fn deserializes_fees() {
        let pos = PersonalPositionState::from_account_data(
            &build_test_data(-100, 200, 1_000_000)
        ).unwrap();
        assert_eq!(pos.fee_growth_inside_0_last_x64, 100);
        assert_eq!(pos.fee_growth_inside_1_last_x64, 200);
        assert_eq!(pos.token_fees_owed_0, 500);
        assert_eq!(pos.token_fees_owed_1, 600);
    }

    #[test]
    fn deserializes_pubkeys() {
        let pos = PersonalPositionState::from_account_data(
            &build_test_data(-100, 200, 1_000_000)
        ).unwrap();
        assert_eq!(pos.nft_mint, Pubkey::new_from_array([1u8; 32]));
        assert_eq!(pos.pool_id,  Pubkey::new_from_array([2u8; 32]));
    }

    #[test]
    fn rejects_too_short() {
        assert!(PersonalPositionState::from_account_data(&[0u8; 4]).is_err());
    }

    #[test]
    fn rejects_truncated_body() {
        let data = [0u8; 18]; // valid discriminator length but body too short
        assert!(PersonalPositionState::from_account_data(&data).is_err());
    }
}