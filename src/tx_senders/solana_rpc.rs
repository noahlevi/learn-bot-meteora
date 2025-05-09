use crate::config::RpcType;
use crate::tx_senders::transaction::{build_transaction_with_config, TransactionConfig};
use crate::tx_senders::{TxResult, TxSender};
use anyhow::Context;
use async_trait::async_trait;
use serde::Serialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use solana_transaction_status::UiTransactionEncoding;
use std::sync::Arc;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::message::VersionedMessage;

#[derive(Clone)]
pub struct GenericRpc {
    pub name: String,
    pub http_rpc: Arc<RpcClient>,
    tx_config: TransactionConfig,
    rpc_type: RpcType,
}

#[derive(Serialize, Debug)]
pub struct TxMetrics {
    pub rpc_name: String,
    pub signature: String,
    pub index: u32,
    pub success: bool,
    pub slot_sent: u64,
    pub slot_landed: Option<u64>,
    pub slot_latency: Option<u64>,
    pub elapsed: Option<u64>, // in milliseconds
}

impl GenericRpc {
    pub fn new(name: String, url: String, config: TransactionConfig, rpc_type: RpcType) -> Self {
        let http_rpc = Arc::new(RpcClient::new(url));
        GenericRpc {
            name,
            http_rpc,
            tx_config: config,
            rpc_type
        }
    }
}

#[async_trait]
impl TxSender for GenericRpc {
    fn name(&self) -> String {
        self.name.clone()
    }

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
    ) -> anyhow::Result<TxResult> {
        println!("SEND SOLANA RPC TX");
        let transaction = build_transaction_with_config(
            &self.tx_config,
            &self.rpc_type,
            recent_blockhash,
            pool,
            user_source_token,
            user_destination_token,
            a_vault,
            b_vault,
            a_token_vault,
            b_token_vault,
            a_vault_lp_mint,
            b_vault_lp_mint,
            a_vault_lp,
            b_vault_lp,
            protocol_token_fee,
            vault_programm,
        );
        let sig = self
            .http_rpc
            .send_transaction_with_config(
                &transaction,
                RpcSendTransactionConfig {
                    skip_preflight: true,
                    preflight_commitment: None,
                    encoding: Some(UiTransactionEncoding::Base64),
                    max_retries: None,
                    min_context_slot: None,
                },
            )
            .await
            .context(format!("Failed to send transaction for {}", self.name))?;
        Ok(TxResult::Signature(sig))
    }
}
