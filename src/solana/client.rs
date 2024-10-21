use std::str::FromStr;

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use solana_transaction_status::{
    EncodedTransaction, UiInstruction, UiMessage, UiParsedInstruction, UiTransactionEncoding,
};
use spl_token::instruction::transfer;

use crate::bridge::TokenClient;

use super::{Builder, Error, NewFromBuilder};

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
            let transaction = self
                .rpc_client
                .get_transaction(&signature, UiTransactionEncoding::Json)
                .unwrap();

            if let EncodedTransaction::Json(transaction_data) = transaction.transaction.transaction
            {
                if let UiMessage::Parsed(message) = transaction_data.message {
                    for instruction in message.instructions.iter() {
                        if let UiInstruction::Parsed(ui_parsed_instruction) = instruction {
                            if let UiParsedInstruction::Parsed(parsed_instruction) =
                                ui_parsed_instruction
                            {
                                if parsed_instruction.program_id == spl_token::id().to_string() {
                                    // Look for TokenInstruction::Transfer
                                    if let Some("transfer") = parsed_instruction
                                        .parsed
                                        .get("type")
                                        .and_then(|t| t.as_str())
                                    {
                                        let info = &parsed_instruction.parsed["info"];
                                        let amount: u64 = info["amount"]
                                            .as_str()
                                            .unwrap_or("0")
                                            .parse()
                                            .unwrap_or(0);
                                        let sender = info["source"].as_str().unwrap();
                                        // recipient should always point to the contract owner, here is just ignore it
                                        // let recipient = info["destination"].as_str().unwrap();
                                        withdrawals.push((
                                            signature.to_string(),
                                            sender.to_owned(),
                                            amount,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // TODO all withdrawals are enumerated, we should check and return the untracked records only
        Ok(withdrawals)
    }
}