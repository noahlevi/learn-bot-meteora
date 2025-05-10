use crate::config::{PingThingsArgs, RpcType};
use crate::tx_senders::constants::{
    BLOXROUTE_TIP_ADDR, JITO_TIP_ADDR, METEORA_PROGRAM_ADDR, NEXTBLOCK_BLOCK_TIP_ADDR, RENT_ADDR,
    SYSTEM_PROGRAM_ADDR, TOKEN_PROGRAM_ADDR,
};
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::v0::Message;
use solana_sdk::message::VersionedMessage;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{EncodableKey, Keypair, Signer};
use solana_sdk::system_instruction;
use solana_sdk::transaction::{Transaction, VersionedTransaction};
use spl_associated_token_account::get_associated_token_address;
use spl_associated_token_account::instruction::create_associated_token_account;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info, warn};

use crate::meteora::SwapData;

#[derive(Clone)]
pub struct TransactionConfig {
    pub keypair: Arc<Keypair>,
    pub compute_unit_limit: u32,
    pub compute_unit_price: u64,
    pub tip: u64,
    pub buy_amount: u64,
    pub min_amount_out: u64,
}

impl From<PingThingsArgs> for TransactionConfig {
    fn from(args: PingThingsArgs) -> Self {
        let keypair = Keypair::from_base58_string(args.private_key.as_str());

        let tip: u64 = (args.tip * LAMPORTS_PER_SOL as f64) as u64;
        let buy_amount: u64 = (args.buy_amount * LAMPORTS_PER_SOL as f64) as u64;
        let min_amount_out: u64 = (args.min_amount_out * 1_000_000 as f64) as u64;

        TransactionConfig {
            keypair: Arc::new(keypair),
            compute_unit_limit: args.compute_unit_limit,
            compute_unit_price: args.compute_unit_price,
            tip: tip,
            buy_amount: buy_amount,
            min_amount_out: min_amount_out,
        }
    }
}

pub fn build_transaction_with_config(
    tx_config: &TransactionConfig,
    rpc_type: &RpcType,
    recent_blockhash: Hash,
    swap_data: SwapData
) -> VersionedTransaction {
    let mut instructions = Vec::new();

    if tx_config.compute_unit_limit > 0 {
        let compute_unit_limit =
            ComputeBudgetInstruction::set_compute_unit_limit(tx_config.compute_unit_limit);
        instructions.push(compute_unit_limit);
    }

    if tx_config.compute_unit_price > 0 {
        let compute_unit_price =
            ComputeBudgetInstruction::set_compute_unit_price(tx_config.compute_unit_price);
        instructions.push(compute_unit_price);
    }

    info!("RPC TYPE {:?}", rpc_type);

    if tx_config.tip > 0 {
        let tip_instruction: Option<Instruction> = match rpc_type {
            RpcType::Jito => Some(system_instruction::transfer(
                &tx_config.keypair.pubkey(),
                &Pubkey::from_str(JITO_TIP_ADDR).unwrap(),
                tx_config.tip,
            )),
            RpcType::Bloxroute => Some(system_instruction::transfer(
                &tx_config.keypair.pubkey(),
                &Pubkey::from_str(BLOXROUTE_TIP_ADDR).unwrap(),
                tx_config.tip,
            )),
            RpcType::Nextblock => Some(system_instruction::transfer(
                &tx_config.keypair.pubkey(),
                &Pubkey::from_str(NEXTBLOCK_BLOCK_TIP_ADDR).unwrap(),
                tx_config.tip,
            )),
            _ => None,
        };

        if tip_instruction.is_some() {
            info!("TIP INSTRUCTION IS SOME");
            instructions.push(tip_instruction.unwrap());
        }
    }

    let token_program_pubkey: Pubkey = Pubkey::from_str(TOKEN_PROGRAM_ADDR).unwrap();

    let owner = tx_config.keypair.pubkey();

    // let token_account_instruction =
    //     create_associated_token_account(&owner, &owner, &token_address, &token_program_pubkey);

    // instructions.push(token_account_instruction);

    // Swap instruction data
    let swap_discriminator = &solana_sdk::hash::hash(b"global:swap").to_bytes()[..8];

    let mut data = vec![];
    data.extend_from_slice(swap_discriminator);
    data.extend_from_slice(&tx_config.buy_amount.to_le_bytes());
    data.extend_from_slice(&tx_config.min_amount_out.to_le_bytes());

    let accounts = vec![
        AccountMeta::new_readonly(swap_data.pool, false),
        AccountMeta::new(swap_data.user_source_token, false),
        AccountMeta::new(swap_data.user_destination_token, false),
        AccountMeta::new(swap_data.a_vault, false),
        AccountMeta::new(swap_data.b_vault, false),
        AccountMeta::new(swap_data.a_token_vault, false),
        AccountMeta::new(swap_data.b_token_vault, false),
        AccountMeta::new(swap_data.a_vault_lp_mint, false),
        AccountMeta::new(swap_data.b_vault_lp_mint, false),
        AccountMeta::new(swap_data.a_vault_lp, false),
        AccountMeta::new(swap_data.b_vault_lp, false),
        AccountMeta::new(swap_data.protocol_token_fee, false),
        AccountMeta::new_readonly(owner, false), //user
        AccountMeta::new_readonly(swap_data.vault_programm, false),
        AccountMeta::new_readonly(token_program_pubkey, false),
    ];

    let swap_instruction = Instruction {
        program_id: Pubkey::from_str(METEORA_PROGRAM_ADDR).unwrap(),
        accounts,
        data,
    };

    instructions.push(swap_instruction);

    let message_v0 =
        Message::try_compile(&owner, instructions.as_slice(), &[], recent_blockhash).unwrap();

    let versioned_message = VersionedMessage::V0(message_v0);

    VersionedTransaction::try_new(versioned_message, &[&tx_config.keypair]).unwrap()
}
