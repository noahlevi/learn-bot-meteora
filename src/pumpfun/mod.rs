use crate::config::PingThingsArgs;
use crate::tx_senders::jito::JitoBundleStatusResponse;
use crate::tx_senders::solana_rpc::TxMetrics;
use crate::tx_senders::transaction::TransactionConfig;
use crate::tx_senders::{create_tx_sender, TxResult, TxSender};

use borsh::{BorshDeserialize, BorshSerialize};
use anyhow::anyhow;
use futures::StreamExt;
use log::debug;
use reqwest::Client;
use serde_json::json;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::signature::Signature;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::transaction::VersionedTransaction;
use solana_transaction_status::TransactionStatusMeta;
use spl_associated_token_account::solana_program::sysvar::instructions;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use crate::bench::Bench;
use crate::core::{extract_instructions};
use crate::tx_senders::constants::PUMP_FUN_PROGRAM_ADDR;

pub const CREATE_IX_DISC: [u8; 8] = [24, 30, 200, 40, 5, 28, 7, 119];
pub const IX_DISCRIMINATOR_SIZE: usize = 8;

#[derive(Debug, BorshDeserialize, BorshSerialize, Clone)]
pub struct CreateIxData {
    pub name: String,
    pub symbol: String,
    pub uri: String
}

pub struct PumpFunController {
    config: PingThingsArgs,
    bench: Bench,

    is_buy: bool,
}

impl PumpFunController {
    pub fn new(config: PingThingsArgs, bench: Bench) -> Self {
        PumpFunController {
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

        let instructions: Vec<(
            solana_sdk::instruction::Instruction
        )> = extract_instructions(meta, transaction.clone())?;

        if !self.is_buy {
            for (instruction) in instructions {
                if instruction.program_id == Pubkey::from_str(PUMP_FUN_PROGRAM_ADDR)? {
                    let ix_discriminator: [u8; 8] = instruction.data[0..IX_DISCRIMINATOR_SIZE].try_into()?;

                    let mut ix_data = &instruction.data[IX_DISCRIMINATOR_SIZE..];

                    let create_ix_data: CreateIxData = BorshDeserialize::deserialize(&mut ix_data)?;

                    if ix_discriminator == CREATE_IX_DISC {
                        info!("create ix: {:?}", create_ix_data);

                        let token_address = instruction.accounts[0].pubkey;
                        let bonding_curve = instruction.accounts[2].pubkey;
                        let associated_bonding_curve = instruction.accounts[3].pubkey;

                        let recent_blockhash: Hash = *transaction.message.recent_blockhash();
                        self.is_buy = true;
                        self.bench.clone().send_buy_tx(recent_blockhash, token_address, bonding_curve, associated_bonding_curve).await;
                    }
                }
            }

        }

        Ok(())
    }
}
