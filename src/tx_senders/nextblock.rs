use crate::config::RpcType;
use crate::meteora::SwapData;
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
use std::str::FromStr;
use tracing::{debug, error, info, warn};

pub struct NextblockTxSender {
    url: String,
    name: String,
    client: Client,
    tx_config: TransactionConfig,
    auth: Option<String>,
}

impl NextblockTxSender {
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
        swap_data: SwapData
    ) -> VersionedTransaction {
        build_transaction_with_config(
            &self.tx_config,
            &RpcType::Bloxroute,
            recent_blockhash,
            swap_data
        )
    }
}

#[derive(Deserialize)]
pub struct NextblockResponse {
    //bundle id is response
    pub signature: String,
    pub uuid: String,
}

#[async_trait]
impl TxSender for NextblockTxSender {
    fn name(&self) -> String {
        self.name.clone()
    }

    async fn send_transaction(
        &self,
        index: u32,
        recent_blockhash: Hash,
        swap_data: SwapData
    ) -> anyhow::Result<TxResult> {
        info!("SEND NEXTBLOCK TX");
        let tx = self.build_transaction_with_config(
            index,
            recent_blockhash,
            swap_data
        );
        let tx_bytes = bincode::serialize(&tx).context("cannot serialize tx to bincode")?;
        let encoded_transaction = base64::encode(tx_bytes);
        let body = json!({
            "transaction": {"content": encoded_transaction},
            "frontRunningProtection": false,
        });

        debug!("sending tx: {}", body.to_string());
        let response = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Authorization", self.auth.clone().unwrap_or("".to_string()))
            .json(&body)
            .send()
            .await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!("failed to send tx: {}", body));
        }
        let parsed_resp = serde_json::from_str::<NextblockResponse>(&body)
            .context("cannot deserialize signature")?;

        Ok(TxResult::Signature(
            Signature::from_str(&parsed_resp.signature).expect("signature from string parsing err"),
        ))
    }
}
