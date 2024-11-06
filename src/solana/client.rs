use std::str::FromStr;
use std::sync::Arc;

use super::{send_token, AnalyzedInstruction, AnalyzedTransaction, Error, TransactionAnalyzer};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction::transfer,
    transaction::Transaction,
};
use solana_transaction_status::UiTransactionEncoding;

pub trait TokenClient {
    type Error: std::fmt::Display + std::fmt::Debug + Send;
    type Address: ToString + FromStr<Err: std::fmt::Debug + Send> + Clone + Send;
    type Amount: Into<u64> + From<u64> + Clone + Send;
    type TxID: ToString + FromStr + Clone + Send;

    /// # Send spl-token to target account
    ///
    /// Arguments:
    /// * recipient_address - The target account from spl-token
    /// * amount - Total amount the authority needs to send
    ///
    /// Returns:
    /// * The signature of the new transaction from solana network
    /// * Otherwise the transaction cannot be made, check the error
    fn send_token(
        &self,
        recipient_address: &Self::Address,
        amount: Self::Amount,
    ) -> anyhow::Result<Self::TxID, Self::Error>;

    /// # Verify a transaction
    /// After the authority receives a withdraw request from DePINC chain, we need
    /// to verify the transaction from solana network also retrieve the number of amount
    ///
    /// Arguments:
    /// * txid - The id of the transaction needs to be verified
    /// * owner - The public-key(or address) of the authority (related token address)
    ///
    /// Returns:
    /// * The amount needs to be transferred on DePINC chain
    /// * Otherwise, the transaction from solana is invalid or it's not a related spl-token tx
    fn verify(&self, signature: &Signature, owner: &Self::Address) -> Result<u64, Self::Error>;
}

#[derive(Clone)]
pub struct SolanaClient {
    pub rpc_client: Arc<RpcClient>,
    authority_key: Arc<Keypair>,
    mint_pubkey: Pubkey,
}

impl SolanaClient {
    pub fn new(
        endpoint: &str,
        mint_pubkey: Pubkey,
        authority_key: Keypair,
        commitment_config: CommitmentConfig,
    ) -> SolanaClient {
        let rpc_client = RpcClient::new_with_commitment(endpoint, commitment_config);
        SolanaClient {
            rpc_client: Arc::new(rpc_client),
            authority_key: Arc::new(authority_key),
            mint_pubkey,
        }
    }

    pub fn send_solana(&self, target_pubkey: &Pubkey, amount: u64) -> Result<Signature, Error> {
        let instruction = transfer(&self.authority_key.pubkey(), target_pubkey, amount);
        let mut transaction =
            Transaction::new_with_payer(&[instruction], Some(&self.authority_key.pubkey()));
        let res = self.rpc_client.get_latest_blockhash();
        if let Err(e) = res {
            println!("cannot get latest block hash, reason: {}", e);
            return Err(Error::CannotGetLatestBlockHash);
        }
        let recent_blockhash = res.unwrap();
        transaction.sign(&[&self.authority_key], recent_blockhash);
        let res = self.rpc_client.send_and_confirm_transaction(&transaction);
        if let Err(e) = res {
            println!("cannot send transaction, reason: {}", e);
            return Err(Error::CannotSendTransaction);
        }
        let signature = res.unwrap();
        Ok(signature)
    }

    pub fn get_transactions_related_to_address(
        &self,
        address: &Pubkey,
    ) -> Result<Vec<AnalyzedTransaction>, Error> {
        let res = self.rpc_client.get_signatures_for_address(address);
        if res.is_err() {
            return Err(Error::CannotGetSignaturesForAddress(address.to_string()));
        }
        let signature_recs = res.unwrap();
        let mut analyzed_transactions = vec![];
        for signature_rec in signature_recs.iter() {
            let signature = Signature::from_str(&signature_rec.signature).unwrap();
            let res = self
                .rpc_client
                .get_transaction(&signature, UiTransactionEncoding::JsonParsed);
            if res.is_err() {
                // cannot retrieve the transaction
                return Err(Error::CannotGetTransactionInfo(
                    signature_rec.signature.clone(),
                ));
            }
            let transaction_meta = res.unwrap();
            let analyzer = TransactionAnalyzer::new(&transaction_meta);
            let res = analyzer.parse(signature, transaction_meta.block_time.unwrap_or(0));
            if res.is_err() {
                todo!("cannot parse the transaction");
            }
            let analyzed_transaction = res.unwrap();
            analyzed_transactions.push(analyzed_transaction);
        }
        Ok(analyzed_transactions)
    }
}

impl TokenClient for SolanaClient {
    type Error = Error;
    type Address = Pubkey;
    type Amount = u64;
    type TxID = Signature;

    fn send_token(
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

    fn verify(&self, signature: &Signature, owner: &Pubkey) -> Result<Self::Amount, Self::Error> {
        let mut amount = 0_u64;
        if let Ok(transaction_meta) = self
            .rpc_client
            .get_transaction(signature, UiTransactionEncoding::JsonParsed)
        {
            let analyzer = TransactionAnalyzer::new(&transaction_meta);
            let res = analyzer.parse(signature.clone(), transaction_meta.block_time.unwrap_or(0));
            if res.is_err() {
                return Err(Error::CannotParseTransactionInfo(signature.to_string()));
            }
            let parsed_transaction = res.unwrap();
            for ix in parsed_transaction.instructions.iter() {
                if let AnalyzedInstruction::SplToken(spl_token_ix) = ix {
                    if spl_token_ix.destination == *owner {
                        amount += spl_token_ix.amount;
                    }
                }
            }
        }
        Ok(amount)
    }
}