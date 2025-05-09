use crate::config::RpcType;
use crate::tx_senders::transaction::{build_transaction_with_config, TransactionConfig};
use crate::tx_senders::{TxResult, TxSender};
use anyhow::Context;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use solana_sdk::bs58;
use solana_sdk::hash::Hash;
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::{Transaction, VersionedTransaction};
use tracing::{error, info, warn};

use std::str::FromStr;
use tracing::debug;

pub struct BloxrouteTxSender {
    url: String,
    name: String,
    client: Client,
    tx_config: TransactionConfig,
    auth: Option<String>,
}

impl BloxrouteTxSender {
    pub fn new(
        name: String,
        url: String,
        tx_config: TransactionConfig,
        client: Client,
        auth: Option<String>,
    ) -> Self {
        Self {
            url,
            name,
            tx_config,
            client,
            auth,
        }
    }

    pub fn build_transaction_with_config(
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
    ) -> VersionedTransaction {
        build_transaction_with_config(
            &self.tx_config,
            &RpcType::Bloxroute,
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
        )
    }
}

#[derive(Deserialize)]
pub struct BloxrouteResponse {
    //bundle id is response
    pub signature: String,
}

#[async_trait]
impl TxSender for BloxrouteTxSender {
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
        println!("SEND BLOXROUTE TX");
        let tx = self.build_transaction_with_config(
            index,
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
        let tx_bytes = bincode::serialize(&tx).context("cannot serialize tx to bincode")?;
        let encoded_transaction = base64::encode(tx_bytes);
        let body = json!({"transaction": {"content": encoded_transaction}});
        debug!("sending tx: {}", body.to_string());
        let response = self
            .client
            .post(&self.url)
            .header("Authorization", self.auth.clone().unwrap_or("".to_string()))
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        let body: String = response.text().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!("failed to send tx: {}", body));
        }
        let parsed_resp = serde_json::from_str::<BloxrouteResponse>(&body)
            .context("cannot deserialize signature")?;

            Ok(TxResult::Signature(
            Signature::from_str(&parsed_resp.signature).expect("signature from string parsing err"),
        ))
    }
}
