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

use super::{inspect_transaction, send_token, Error};

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
        recipient_address: &Self::Address,
        amount: Self::Amount,
    ) -> Result<Self::TxID, Self::Error> {
        let signature = send_token(
            &self.rpc_client,
            &self.mint_pubkey,
            &self.authority_key,
            recipient_address,
            amount,
        )?;
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
