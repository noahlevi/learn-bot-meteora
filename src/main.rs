use std::str::FromStr;
use async_trait::async_trait;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey, pubkey::Pubkey, system_instruction};
use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
};
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use solana_sdk::hash::Hash;
use solana_sdk::instruction::{AccountMeta, CompiledInstruction, Instruction};
use solana_sdk::message::{Message, VersionedMessage};
use solana_sdk::signature::Signer;
use solana_sdk::signature::Keypair;
use solana_sdk::system_instruction::SystemInstruction;
use solana_sdk::sysvar::recent_blockhashes::RecentBlockhashes;
use spl_associated_token_account::get_associated_token_address;
use spl_associated_token_account::instruction::create_associated_token_account;
use tokio::signal;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequestFilterAccounts, SubscribeRequestFilterTransactions,
};
use tracing::info;
use crate::bench::Bench;
use crate::geyser::{GeyserResult, YellowstoneGrpcGeyser, YellowstoneGrpcGeyserClient};
use crate::config::PingThingsArgs;
// use crate::pumpfun::PumpFunController;
use crate::meteora::MeteoraController;

pub const PUMPFUN_PROGRAM_ID: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const METEORA_PROGRAM_ID: Pubkey = pubkey!("Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB");
pub const WSOL_ACCOUNT_ID: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

mod bench;
mod config;
mod tx_senders;
mod geyser;
// mod pumpfun;
mod core;
mod meteora;

#[tokio::main]
pub async fn main() -> GeyserResult<()> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish(),
    )
        .unwrap();

    let config_controller: PingThingsArgs = PingThingsArgs::new();
    let bench_controller: Bench = Bench::new(config_controller.clone());

    let meteora_controller: MeteoraController = MeteoraController::new(config_controller.clone(), bench_controller.clone());

    info!("starting with config {:?}", config_controller);

    env_logger::init();
    dotenv::dotenv().ok();

    let account_filters: HashMap<String, SubscribeRequestFilterAccounts> = HashMap::new();

    let transaction_filter = SubscribeRequestFilterTransactions {
        vote: Some(false),
        failed: Some(false),
        account_include: vec![METEORA_PROGRAM_ID.to_string().clone()],
        account_exclude: vec![],
        account_required: vec![
            WSOL_ACCOUNT_ID.to_string().clone()
            ],
        signature: None,
    };

    let mut transaction_filters: HashMap<String, SubscribeRequestFilterTransactions> =
        HashMap::new();

    transaction_filters.insert("meteora_transaction_filter".to_string(), transaction_filter);

    let yellowstone_grpc = YellowstoneGrpcGeyserClient::new(
        config_controller.geyser_url,
        Some(config_controller.geyser_x_token),
        Some(CommitmentLevel::Processed),
        account_filters,
        transaction_filters,
        Arc::new(RwLock::new(HashSet::new())),
    );

    let _ = yellowstone_grpc.consume(meteora_controller).await;
    Ok(())
}
