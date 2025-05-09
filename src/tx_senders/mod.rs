use crate::config::{RpcConfig, RpcType};
use crate::tx_senders::bloxroute::BloxrouteTxSender;
use crate::tx_senders::jito::JitoTxSender;
use crate::tx_senders::nextblock::NextblockTxSender;
use crate::tx_senders::solana_rpc::GenericRpc;
use crate::tx_senders::transaction::TransactionConfig;
use async_trait::async_trait;
use reqwest::Client;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::sync::Arc;
use tracing::{error, info, warn};

pub mod bloxroute;
pub mod constants;
pub mod jito;
pub mod nextblock;
pub mod solana_rpc;
pub mod transaction;

#[derive(Debug, Clone)]
pub enum TxResult {
    Signature(Signature),
    BundleID(String),
}

impl Into<String> for TxResult {
    fn into(self) -> String {
        match self {
            TxResult::Signature(sig) => sig.to_string(),
            TxResult::BundleID(bundle_id) => bundle_id,
        }
    }
}

#[async_trait]
pub trait TxSender: Sync + Send {
    fn name(&self) -> String;
    async fn send_transaction(
        &self,
        index: u32,
        recent_blockhash: Hash,
        pool: Pubkey,
        user_source_token: Pubkey,
        user_destination_token: Pubkey,
        a_vault: Pubkey,
        b_vault: Pubkey,
        a_token_vault: Pubkey,
        b_token_vault: Pubkey,
        a_vault_lp_mint: Pubkey,
        b_vault_lp_mint: Pubkey,
        a_vault_lp: Pubkey,
        b_vault_lp: Pubkey,
        protocol_token_fee: Pubkey,
        vault_programm: Pubkey,
    ) -> anyhow::Result<TxResult>;
}

pub fn create_tx_sender(
    name: String,
    rpc_config: RpcConfig,
    tx_config: TransactionConfig,
    client: Client,
) -> Arc<dyn TxSender> {
    info!("create_tx_sender {:?}", rpc_config.rpc_type);
    match rpc_config.rpc_type {
        RpcType::SolanaRpc => {
            let tx_sender = GenericRpc::new(name, rpc_config.url, tx_config, RpcType::SolanaRpc);
            Arc::new(tx_sender)
        }
        RpcType::Jito => {
            let tx_sender = JitoTxSender::new(name, rpc_config.url, tx_config, client);
            Arc::new(tx_sender)
        }
        RpcType::Bloxroute => {
            let tx_sender = BloxrouteTxSender::new(
                name,
                rpc_config.url,
                tx_config,
                client,
                rpc_config.auth.clone(),
            );
            Arc::new(tx_sender)
        }
        RpcType::Nextblock => {
            let tx_sender = NextblockTxSender::new(
                name,
                rpc_config.url,
                tx_config,
                client,
                rpc_config.auth.clone(),
            );
            Arc::new(tx_sender)
        }
    }
}
