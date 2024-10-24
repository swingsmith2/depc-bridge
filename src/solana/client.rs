use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};

use crate::bridge::TokenClient;

use super::{inspect_transaction, send_token, Error};

pub struct SolanaClient {
    rpc_client: RpcClient,
    authority_key: Keypair,
    mint_pubkey: Pubkey,
}

impl SolanaClient {
    pub fn new(endpoint: &str, mint_pubkey: Pubkey, authority_key: Keypair) -> SolanaClient {
        let rpc_client = RpcClient::new_with_commitment(endpoint, CommitmentConfig::confirmed());
        SolanaClient {
            rpc_client,
            authority_key,
            mint_pubkey,
        }
    }
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
        // fetch signatures of transactions involving this token account
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

        Ok(withdrawals)
    }
}
