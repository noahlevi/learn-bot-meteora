use crate::config::PingThingsArgs;
use crate::meteora::SwapData;
use crate::tx_senders::jito::JitoBundleStatusResponse;
use crate::tx_senders::solana_rpc::TxMetrics;
use crate::tx_senders::transaction::TransactionConfig;
use crate::tx_senders::{create_tx_sender, TxResult, TxSender};
use anyhow::anyhow;
use futures::StreamExt;
use log::debug;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct Bench {
    config: PingThingsArgs,
    tx_subscribe_sender: tokio::sync::mpsc::Sender<TxMetrics>,
    rpcs: Vec<Arc<dyn TxSender>>,
    client: Client,
}

impl Bench {
    pub fn new(config: PingThingsArgs) -> Self {
        let (tx_subscribe_sender, tx_subscribe_receiver) = tokio::sync::mpsc::channel(100);
        let tx_config: TransactionConfig = config.clone().into();
        let client = Client::new();

        let rpcs = config
            .rpc
            .clone()
            .into_iter()
            .map(|(name, rpc)| create_tx_sender(name, rpc, tx_config.clone(), client.clone()))
            .collect::<Vec<Arc<dyn TxSender>>>();

        Bench {
            config,
            tx_subscribe_sender,
            rpcs,
            client,
        }
    }

    pub async fn send_and_confirm_transaction(
        tx_index: u32,
        rpc_sender: Arc<dyn TxSender>,
        recent_blockhash: Hash,
        swap_data: SwapData
    ) -> anyhow::Result<()> {
        let start = tokio::time::Instant::now();

        let tx_result = rpc_sender
            .send_transaction(
                tx_index,
                recent_blockhash,
                swap_data
            )
            .await?;

        info!(
            "complete rpc: {:?} {:?} ms",
            rpc_sender.name(),
            start.elapsed().as_millis() as u64
        );
        Ok(())
    }

    pub async fn send_swap_tx(
        self,
        recent_blockhash: Hash,
        swap_data: SwapData
    ) {
        tokio::select! {
            _ = self.send_swap_tx_inner(
                recent_blockhash,
                swap_data
            ) => {}
        }
    }

    async fn send_swap_tx_inner(
        self,
        recent_blockhash: Hash,
        swap_data: SwapData
    ) {
        let start = tokio::time::Instant::now();
        info!("starting create buy tx");
        let mut tx_handles = Vec::new();

        for rpc in &self.rpcs {
            let rpc_name = rpc.name();
            let rpc_sender = rpc.clone();
            let client = self.client.clone();
            let hdl = tokio::spawn(async move {
                let index = 0;
                if let Err(e) = Self::send_and_confirm_transaction(
                    index,
                    rpc_sender,
                    recent_blockhash,
                    swap_data
                )
                .await
                {
                    error!("error end_and_confirm_transaction {:?}", e);
                }
            });
            tx_handles.push(hdl);
        }
        info!("waiting for transactions to complete...");

        // wait for all transactions to complete
        for hdl in tx_handles {
            hdl.await.unwrap_or_default();
        }

        info!(
            "bench complete! {:?} ms",
            start.elapsed().as_millis() as u64
        );
    }
}
