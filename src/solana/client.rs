use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use spl_token::instruction::transfer;

use crate::{
    bridge::TokenClient,
    solana::{get_or_create_associated_token_account, wait_transaction_until_processed},
};

use super::{inspect_transaction, Error};

pub struct SolanaClient {
    rpc_client: RpcClient,
    authority_key: Keypair,
    mint_pubkey: Pubkey,
}

impl TokenClient for SolanaClient {
    type Error = Error;
    type Address = Pubkey;
    type Amount = u64;
    type TxID = Signature;

    fn send(
        &self,
        sender_address: Self::Address,
        recipient_address: Self::Address,
        amount: Self::Amount,
    ) -> Result<Self::TxID, Self::Error> {
        // we need to retrieve the associated token address
        let res = get_or_create_associated_token_account(
            &self.rpc_client,
            &self.mint_pubkey,
            &self.authority_key,
        );
        if res.is_err() {
            return Err(Error::CannotGetAssociatedAccount);
        }
        let (sender, signature_opt) = res.unwrap();
        println!("got sender ({}) token address: {}", sender_address, sender);
        if let Some(signature) = signature_opt {
            wait_transaction_until_processed(&self.rpc_client, &signature).unwrap();
        }
        let res = get_or_create_associated_token_account(
            &self.rpc_client,
            &self.mint_pubkey,
            &self.authority_key,
        );
        if res.is_err() {
            return Err(Error::CannotGetAssociatedAccount);
        }
        let (recipient, signature_opt) = res.unwrap();
        println!(
            "got recipient ({}) token address: {}",
            recipient_address, recipient
        );
        if let Some(signature) = signature_opt {
            wait_transaction_until_processed(&self.rpc_client, &signature).unwrap();
        }
        // Create the SPL token transfer instruction
        let authority_pubkey = self.authority_key.pubkey();
        let transfer_instruction = transfer(
            &spl_token::id(),
            &sender,
            &recipient,
            &authority_pubkey,
            &[],
            amount,
        )
        .unwrap();

        // Create a new transaction
        let mut transaction =
            Transaction::new_with_payer(&[transfer_instruction], Some(&authority_pubkey));

        // Get recent blockhash
        let recent_blockhash = self.rpc_client.get_latest_blockhash().unwrap();
        transaction.sign(&[&self.authority_key], recent_blockhash);

        // Send the transaction
        let signature = self
            .rpc_client
            .send_and_confirm_transaction(&transaction)
            .unwrap();
        Ok(signature)
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
            let transactions = inspect_transaction(&self.rpc_client, signature)?;
            for transaction in transactions.iter() {
                withdrawals.push((
                    transaction.signature,
                    transaction.instruction.source,
                    transaction.instruction.amount,
                ))
            }
        }

        // TODO all withdrawals are enumerated, we should check and return the untracked records only
        Ok(withdrawals)
    }
}
