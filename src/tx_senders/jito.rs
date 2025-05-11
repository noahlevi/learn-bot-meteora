use crate::config::RpcType;
use crate::meteora::SwapData;
use crate::tx_senders::transaction::{build_transaction_with_config, TransactionConfig};
use crate::tx_senders::{TxResult, TxSender};
use anyhow::Context;
use async_trait::async_trait;
use log::info;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use solana_sdk::bs58;
use solana_sdk::hash::Hash;
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::{Transaction, VersionedTransaction};
use tracing::debug;

pub struct JitoTxSender {
    url: String,
    name: String,
    client: Client,
    tx_config: TransactionConfig,
}

impl JitoTxSender {
    pub fn new(name: String, url: String, tx_config: TransactionConfig, client: Client) -> Self {
        Self {
            url,
            name,
            tx_config,
            client,
        }
    }

    pub fn build_transaction_with_config(
        &self,
        index: u32,
        recent_blockhash: Hash,
        swap_data: SwapData,
    ) -> VersionedTransaction {
        build_transaction_with_config(&self.tx_config, &RpcType::Jito, recent_blockhash, swap_data)
    }
}

#[derive(Deserialize)]
pub struct JitoBundleStatusResponseInnerContext {
    pub slot: u64,
}

#[derive(Deserialize)]
pub struct JitoBundleStatusResponseInnerValue {
    pub slot: u64,
    pub bundle_id: String,
    pub transactions: Vec<String>,
    pub confirmation_status: String,
    pub err: Value,
}

#[derive(Deserialize)]
pub struct JitoBundleStatusResponseInner {
    pub context: JitoBundleStatusResponseInnerContext,
    pub value: Vec<JitoBundleStatusResponseInnerValue>,
}
#[derive(Deserialize)]
pub struct JitoBundleStatusResponse {
    pub result: JitoBundleStatusResponseInner,
}

#[derive(Deserialize)]
pub struct JitoResponse {
    //bundle id is response
    pub result: String,
}

#[async_trait]
impl TxSender for JitoTxSender {
    fn name(&self) -> String {
        self.name.clone()
    }

    async fn send_transaction(
        &self,
        index: u32,
        recent_blockhash: Hash,
        swap_data: SwapData,
    ) -> anyhow::Result<TxResult> {
        info!("SEND JITO TX");
        let tx = self.build_transaction_with_config(index, recent_blockhash, swap_data);
        let tx_bytes = bincode::serialize(&tx).context("cannot serialize tx to bincode")?;
        let encoded_transaction = bs58::encode(tx_bytes).into_string();
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [encoded_transaction]
        });
        debug!("sending tx: {}", body.to_string());
        let response = self.client.post(&self.url).json(&body).send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!("failed to send tx: {}", body));
        }

        let parsed_resp =
            serde_json::from_str::<JitoResponse>(&body).context("cannot deserialize signature")?;

        Ok(TxResult::BundleID(parsed_resp.result))
    }
}
