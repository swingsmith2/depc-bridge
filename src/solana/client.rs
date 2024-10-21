use std::str::FromStr;

use serde_json::Value;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use solana_transaction_status::{
    parse_instruction::ParsedInstruction, EncodedTransaction, UiInstruction, UiMessage,
    UiParsedInstruction, UiParsedMessage, UiTransaction, UiTransactionEncoding,
};
use spl_token::instruction::transfer;

use crate::bridge::TokenClient;

use super::{Builder, Error, NewFromBuilder};

pub struct TransactionInfo {
    signature: Signature,
    instruction: InstructionInfo,
}

pub struct InstructionInfo {
    amount: u64,
    authority: Pubkey,
    destination: Pubkey,
    source: Pubkey,
    owner: Pubkey,
}

pub struct Client {
    rpc_client: RpcClient,
    authority_key: Keypair,
    mint_pubkey: Pubkey,
}

impl NewFromBuilder for Client {
    type T = Client;

    fn new_from_builder(builder: Builder) -> Result<Self::T, Error> {
        let rpc_client = builder.create_rpc_client_from_url()?;
        if builder.authority_key.is_none() {
            return Err(Error::MissingRequiredField);
        }
        let authority_key = builder.authority_key.unwrap();
        if builder.mint_pubkey.is_none() {
            return Err(Error::MissingRequiredField);
        }
        let mint_pubkey = builder.mint_pubkey.unwrap();
        Ok(Client {
            rpc_client,
            authority_key,
            mint_pubkey,
        })
    }
}

impl TokenClient for Client {
    type Error = Error;
    type Address = String;
    type Amount = u64;
    type TxID = String;

    fn send(
        &self,
        sender_address: Self::Address,
        recipient_address: Self::Address,
        amount: Self::Amount,
    ) -> Result<Self::TxID, Self::Error> {
        // Define the sender's token account, recipient, and the token mint
        let sender_token_account = Pubkey::from_str(&sender_address).unwrap();
        let recipient_token_account = Pubkey::from_str(&recipient_address).unwrap();

        // Create the SPL token transfer instruction
        let transfer_instruction = transfer(
            &self.mint_pubkey,
            &sender_token_account,        // Sender's token account
            &recipient_token_account,     // Recipient's token account
            &self.authority_key.pubkey(), // Authority of the sender (usually the owner's public key)
            &[],                          // Signers (empty if the sender is the payer/owner)
            amount,                       // Amount to transfer
        )
        .unwrap();

        // Create a new transaction
        let mut transaction = Transaction::new_with_payer(
            &[transfer_instruction],            // Instructions for the transfer
            Some(&self.authority_key.pubkey()), // Payer for transaction fees
        );

        // Get recent blockhash
        let recent_blockhash = self.rpc_client.get_latest_blockhash().unwrap();
        transaction.sign(&[&self.authority_key], recent_blockhash);

        // Send the transaction
        let signature = self
            .rpc_client
            .send_and_confirm_transaction(&transaction)
            .unwrap();
        Ok(signature.to_string())
    }

    fn load_unfinished_withdrawals(
        &self,
    ) -> Result<Vec<(Self::TxID, Self::Address, Self::Amount)>, Self::Error> {
        // Fetch signatures of transactions involving this token account
        let signatures = self
            .rpc_client
            .get_signatures_for_address(&self.authority_key.pubkey())
            .unwrap();

        let mut withdrawals = vec![];

        for signature_info in signatures.iter() {
            let signature = signature_info.signature.parse::<Signature>().unwrap();
            let transactions = self.inspect_transaction(signature)?;
            for transaction in transactions.iter() {
                withdrawals.push((
                    transaction.signature.to_string(),
                    transaction.instruction.source.to_string(),
                    transaction.instruction.amount,
                ));
            }
        }

        // TODO all withdrawals are enumerated, we should check and return the untracked records only
        Ok(withdrawals)
    }
}

impl Client {
    pub fn inspect_transaction(&self, signature: Signature) -> Result<Vec<TransactionInfo>, Error> {
        let res = self
            .rpc_client
            .get_transaction(&signature, UiTransactionEncoding::Json);
        if res.is_err() {
            return Err(Error::CannotGetTransactionInfo);
        }
        let json = res.unwrap();
        let mut transactions = vec![];
        if let EncodedTransaction::Json(transaction) = json.transaction.transaction {
            let instructions = parse_spl_token_instruction(&transaction)?;
            for instruction in instructions.iter() {
                if let Some(transaction_info) = parse_instruction(signature, *instruction)? {
                    transactions.push(transaction_info);
                }
            }
        }
        Ok(transactions)
    }
}

fn parse_ui_message(ui_message: &UiMessage) -> Result<&UiParsedMessage, Error> {
    if let UiMessage::Parsed(message) = ui_message {
        Ok(message)
    } else {
        Err(Error::ExtractMismatchedType)
    }
}

fn parse_ui_instruction(ui_instruction: &UiInstruction) -> Result<&UiParsedInstruction, Error> {
    if let UiInstruction::Parsed(instruction) = ui_instruction {
        Ok(instruction)
    } else {
        Err(Error::ExtractMismatchedType)
    }
}

fn parse_instruction_from_ui_parsed_instruction(
    instruction: &UiParsedInstruction,
) -> Result<&ParsedInstruction, Error> {
    if let UiParsedInstruction::Parsed(instruction) = instruction {
        Ok(instruction)
    } else {
        Err(Error::ExtractMismatchedType)
    }
}

fn parse_spl_token_instruction(
    transaction: &UiTransaction,
) -> Result<Vec<&ParsedInstruction>, Error> {
    let mut instructions = vec![];
    let message = parse_ui_message(&transaction.message)?;
    for instruction in message.instructions.iter() {
        let instruction = parse_ui_instruction(instruction)?;
        let instruction = parse_instruction_from_ui_parsed_instruction(instruction)?;
        if instruction.program_id == spl_token::id().to_string() {
            // ok, this is spl_token instruction
            instructions.push(instruction);
        }
    }
    Ok(instructions)
}

fn parse_instruction_info(value: &Value) -> Result<InstructionInfo, Error> {
    let amount: u64 = value["amount"].as_str().unwrap_or("0").parse().unwrap_or(0);
    let authority = Pubkey::try_from(value["authority"].as_str().unwrap()).unwrap();
    let destination = Pubkey::try_from(value["destination"].as_str().unwrap()).unwrap();
    let source = Pubkey::try_from(value["source"].as_str().unwrap()).unwrap();
    let owner = Pubkey::try_from(value["owner"].as_str().unwrap()).unwrap();
    Ok(InstructionInfo {
        amount,
        authority,
        destination,
        source,
        owner,
    })
}

fn parse_instruction(
    signature: Signature,
    parsed_instruction: &ParsedInstruction,
) -> Result<Option<TransactionInfo>, Error> {
    // Look for TokenInstruction::Transfer
    if let Some("transfer") = parsed_instruction
        .parsed
        .get("type")
        .and_then(|t| t.as_str())
    {
        let value = &parsed_instruction.parsed["info"];
        let instruction = parse_instruction_info(&value)?;
        Ok(Some(TransactionInfo {
            signature,
            instruction,
        }))
    } else {
        Ok(None)
    }
}
