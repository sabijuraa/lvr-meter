use anyhow::{Context, Result};
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_account_decoder::UiAccountEncoding;
use solana_sdk::{commitment_config::CommitmentConfig, program_pack::Pack, pubkey::Pubkey};
use std::str::FromStr;

use crate::config::WalletAddress;
use crate::fetcher::raydium::types::{
    PersonalPositionState, PERSONAL_POSITION_DISCRIMINATOR, RAYDIUM_CLMM_PROGRAM_ID,
};
use crate::fetcher::rpc::RpcClientWrapper;

/// SPL Token program ID
const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

/// Fetch all Raydium CLMM positions held by a wallet.
///
/// Flow:
///   1. Get all SPL token accounts owned by the wallet (NFT mints with amount=1)
///   2. For each NFT mint, derive the PersonalPosition PDA
///   3. Fetch and deserialize all matching accounts
pub fn fetch_positions(
    wallet: &WalletAddress,
    client: &RpcClientWrapper,
) -> Result<Vec<PersonalPositionState>> {
    let wallet_pubkey = Pubkey::from_str(wallet.as_str())
        .context("Invalid wallet pubkey")?;

    let nft_mints = fetch_nft_mints_for_wallet(&client.client, &wallet_pubkey)?;

    if nft_mints.is_empty() {
        return Ok(vec![]);
    }

    let positions = fetch_positions_by_discriminator(&client.client)?;

    let wallet_positions: Vec<PersonalPositionState> = positions
        .into_iter()
        .filter(|p| nft_mints.contains(&p.nft_mint))
        .collect();

    Ok(wallet_positions)
}

/// Get all NFT mints (amount=1 tokens) held by a wallet
fn fetch_nft_mints_for_wallet(
    client: &RpcClient,
    wallet: &Pubkey,
) -> Result<Vec<Pubkey>> {
    let token_program = Pubkey::from_str(TOKEN_PROGRAM_ID).unwrap();

    let accounts = client
        .get_token_accounts_by_owner(
            wallet,
            solana_client::rpc_request::TokenAccountsFilter::ProgramId(token_program),
        )
        .context("Failed to fetch token accounts for wallet")?;

    let mut nft_mints = Vec::new();

    for keyed_account in accounts {
        if let solana_account_decoder::UiAccountData::Json(parsed) =
            &keyed_account.account.data
        {
            if let Some(info) = parsed.parsed
                .get("info")
            {
                let amount = info
                    .get("tokenAmount")
                    .and_then(|ta| ta.get("amount"))
                    .and_then(|a| a.as_str())
                    .unwrap_or("0");

                // NFTs have amount=1 and decimals=0
                let decimals = info
                    .get("tokenAmount")
                    .and_then(|ta| ta.get("decimals"))
                    .and_then(|d| d.as_u64())
                    .unwrap_or(1);

                if amount == "1" && decimals == 0 {
                    if let Some(mint_str) = info.get("mint").and_then(|m| m.as_str()) {
                        if let Ok(mint) = Pubkey::from_str(mint_str) {
                            nft_mints.push(mint);
                        }
                    }
                }
            }
        }
    }

    Ok(nft_mints)
}

/// Fetch all PersonalPosition accounts from Raydium CLMM program
/// filtered by the account discriminator
fn fetch_positions_by_discriminator(
    client: &RpcClient,
) -> Result<Vec<PersonalPositionState>> {
    let program_id = Pubkey::from_str(RAYDIUM_CLMM_PROGRAM_ID).unwrap();

    let filters = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            PERSONAL_POSITION_DISCRIMINATOR.to_vec(),
        )),
    ];

    let config = RpcProgramAccountsConfig {
        filters: Some(filters),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        },
        with_context: None,
    };

    let accounts = client
        .get_program_accounts_with_config(&program_id, config)
        .context("getProgramAccounts failed for Raydium CLMM")?;

    let mut positions = Vec::new();

    for (_pubkey, account) in accounts {
        match PersonalPositionState::from_account_data(&account.data) {
            Ok(pos) => positions.push(pos),
            Err(e) => tracing::warn!("Failed to deserialize position account: {}", e),
        }
    }

    Ok(positions)
}