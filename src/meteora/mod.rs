use crate::config::PingThingsArgs;
use crate::tx_senders::jito::JitoBundleStatusResponse;
use crate::tx_senders::solana_rpc::TxMetrics;
use crate::tx_senders::transaction::TransactionConfig;
use crate::tx_senders::{create_tx_sender, TxResult, TxSender};

use crate::bench::Bench;
use crate::core::extract_instructions;
use crate::tx_senders::constants::METEORA_PROGRAM_ADDR;
use crate::WSOL_ACCOUNT_ID;
use anyhow::anyhow;
use borsh::{BorshDeserialize, BorshSerialize};
use futures::StreamExt;
use log::debug;
use reqwest::Client;
use serde_json::json;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::TransactionStatusMeta;
use spl_associated_token_account::solana_program::sysvar::instructions;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

pub const INITIALIZE_PERMISSIONLESS_CONSTANT_PRODUCT_POOL_WITH_CONFIG_2_DISC: [u8; 8] =
    [48, 149, 220, 130, 61, 11, 9, 178];
pub const IX_DISCRIMINATOR_SIZE: usize = 8;

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub struct AddLiquidityIxData {
    pub tokenAAmount: u64,
    pub tokenBAmount: u64,
    pub activationPoint: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub struct SwapData {
    pub pool: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub a_vault: Pubkey,
    pub b_vault: Pubkey,
    pub a_token_vault: Pubkey,
    pub b_token_vault: Pubkey,
    pub a_vault_lp_mint: Pubkey,
    pub b_vault_lp_mint: Pubkey,
    pub a_vault_lp: Pubkey,
    pub b_vault_lp: Pubkey,
    pub vault_programm: Pubkey,
    pub protocol_token_a_fee: Pubkey,
    pub protocol_token_b_fee: Pubkey,
}

impl SwapData {
    pub fn new(
        pool: Pubkey,
        token_a_mint: Pubkey,
        token_b_mint: Pubkey,
        a_vault: Pubkey,
        b_vault: Pubkey,
        a_token_vault: Pubkey,
        b_token_vault: Pubkey,
        a_vault_lp_mint: Pubkey,
        b_vault_lp_mint: Pubkey,
        a_vault_lp: Pubkey,
        b_vault_lp: Pubkey,
        vault_programm: Pubkey,
        protocol_token_a_fee: Pubkey,
        protocol_token_b_fee: Pubkey,
    ) -> SwapData {
        Self {
            pool,
            token_a_mint,
            token_b_mint,
            a_vault,
            b_vault,
            a_token_vault,
            b_token_vault,
            a_vault_lp_mint,
            b_vault_lp_mint,
            a_vault_lp,
            b_vault_lp,
            vault_programm,
            protocol_token_a_fee,
            protocol_token_b_fee,
        }
    }
}

pub struct MeteoraController {
    config: PingThingsArgs,
    bench: Bench,

    is_buy: bool,
}

impl MeteoraController {
    pub fn new(config: PingThingsArgs, bench: Bench) -> Self {
        MeteoraController {
            config,
            bench: bench,
            is_buy: false,
        }
    }

    pub async fn transaction_handler(
        &mut self,
        signature: Signature,
        transaction: VersionedTransaction,
        meta: TransactionStatusMeta,
        is_vote: bool,
        slot: u64,
    ) -> anyhow::Result<()> {
        // info!("INSIDE meteora tx handler");
        let instructions: Vec<(solana_sdk::instruction::Instruction)> =
            extract_instructions(meta, transaction.clone())?;

        if !self.is_buy {
            for (instruction) in instructions {
                if instruction.program_id == Pubkey::from_str(METEORA_PROGRAM_ADDR)? {
                    let ix_discriminator: [u8; 8] =
                        instruction.data[0..IX_DISCRIMINATOR_SIZE].try_into()?;

                    // info!("INSIDE meteora tx handler {:?}", ix_discriminator);

                    let mut ix_data = &instruction.data[IX_DISCRIMINATOR_SIZE..];

                    let create_ix_data: AddLiquidityIxData =
                        BorshDeserialize::deserialize(&mut ix_data)?;

                    if ix_discriminator
                        == INITIALIZE_PERMISSIONLESS_CONSTANT_PRODUCT_POOL_WITH_CONFIG_2_DISC
                    {
                        info!("create ix: {:?}", create_ix_data);

                        let pool = instruction.accounts[0].pubkey;

                        let token_a_mint = instruction.accounts[3].pubkey;
                        let token_b_mint = instruction.accounts[4].pubkey;

                        if vec![token_a_mint, token_b_mint].contains(&WSOL_ACCOUNT_ID) {
                            info!("create ix: {:?}", create_ix_data);

                            let a_vault = instruction.accounts[5].pubkey;
                            let b_vault = instruction.accounts[6].pubkey;

                            let a_token_vault = instruction.accounts[7].pubkey;
                            let b_token_vault = instruction.accounts[8].pubkey;

                            let a_vault_lp_mint = instruction.accounts[9].pubkey;
                            let b_vault_lp_mint = instruction.accounts[10].pubkey;

                            let a_vault_lp = instruction.accounts[11].pubkey;
                            let b_vault_lp = instruction.accounts[12].pubkey;

                            let protocol_token_a_fee = instruction.accounts[16].pubkey;
                            let protocol_token_b_fee = instruction.accounts[17].pubkey;
                            let vault_programm = instruction.accounts[22].pubkey;

                            let swap_data: SwapData = SwapData::new(
                                pool,
                                token_a_mint,
                                token_b_mint,
                                a_vault,
                                b_vault,
                                a_token_vault,
                                b_token_vault,
                                a_vault_lp_mint,
                                b_vault_lp_mint,
                                a_vault_lp,
                                b_vault_lp,
                                vault_programm,
                                protocol_token_a_fee,
                                protocol_token_b_fee,
                            );

                            let recent_blockhash: Hash = *transaction.message.recent_blockhash();
                            self.is_buy = true;
                            self.bench
                                .clone()
                                .send_swap_tx(recent_blockhash, swap_data)
                                .await;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
