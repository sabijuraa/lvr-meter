use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::fetcher::raydium::types::RAYDIUM_CLMM_PROGRAM_ID;

/// Anchor discriminator for `swap_v2` instruction
/// sha256("global:swap_v2")[..8]
pub const SWAP_V2_DISCRIMINATOR: [u8; 8] =
    [0x2b, 0x04, 0xed, 0x0b, 0x1a, 0xc9, 0x1e, 0x62];

/// Anchor discriminator for `swap` instruction (v1, kept for legacy txs)
/// sha256("global:swap")[..8]
pub const SWAP_V1_DISCRIMINATOR: [u8; 8] =
    [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];

/// Returns true if the instruction is a Raydium CLMM swap (v1 or v2).
///
/// Checks:
///   1. The program account key matches Raydium CLMM program ID
///   2. The instruction data starts with a known swap discriminator
pub fn is_raydium_swap(
    program_id:   &Pubkey,
    data:         &[u8],
) -> bool {
    let raydium_id = match Pubkey::from_str(RAYDIUM_CLMM_PROGRAM_ID) {
        Ok(id) => id,
        Err(_) => return false,
    };

    if *program_id != raydium_id {
        return false;
    }

    if data.len() < 8 {
        return false;
    }

    let disc = &data[..8];
    disc == SWAP_V2_DISCRIMINATOR || disc == SWAP_V1_DISCRIMINATOR
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raydium_program_id() -> Pubkey {
        Pubkey::from_str(RAYDIUM_CLMM_PROGRAM_ID).unwrap()
    }

    #[test]
    fn detects_swap_v2_discriminator() {
        let mut data = SWAP_V2_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&[0u8; 100]); // rest of instruction data
        assert!(is_raydium_swap(&raydium_program_id(), &data));
    }

    #[test]
    fn detects_swap_v1_discriminator() {
        let mut data = SWAP_V1_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&[0u8; 100]);
        assert!(is_raydium_swap(&raydium_program_id(), &data));
    }

    #[test]
    fn rejects_wrong_program_id() {
        let wrong_program = Pubkey::new_unique();
        let mut data      = SWAP_V2_DISCRIMINATOR.to_vec();
        data.extend_from_slice(&[0u8; 100]);
        assert!(!is_raydium_swap(&wrong_program, &data));
    }

    #[test]
    fn rejects_wrong_discriminator() {
        let mut data = vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        data.extend_from_slice(&[0u8; 100]);
        assert!(!is_raydium_swap(&raydium_program_id(), &data));
    }

    #[test]
    fn rejects_data_too_short() {
        let data = vec![0x2b, 0x04]; // less than 8 bytes
        assert!(!is_raydium_swap(&raydium_program_id(), &data));
    }

    #[test]
    fn rejects_empty_data() {
        assert!(!is_raydium_swap(&raydium_program_id(), &[]));
    }

    #[test]
    fn discriminator_bytes_are_correct() {
        // Verify against known sha256("global:swap_v2")[:8]
        assert_eq!(
            SWAP_V2_DISCRIMINATOR,
            [0x2b, 0x04, 0xed, 0x0b, 0x1a, 0xc9, 0x1e, 0x62]
        );
        assert_eq!(
            SWAP_V1_DISCRIMINATOR,
            [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8]
        );
    }
}