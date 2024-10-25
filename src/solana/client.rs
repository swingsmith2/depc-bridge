use solana_client::rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction::transfer,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;

use crate::{bridge::TokenClient, solana::parse_transaction};

use super::{send_token, Error};

pub struct SolanaClient {
    rpc_client: RpcClient,
    commitment_config: CommitmentConfig,
    authority_key: Keypair,
    mint_pubkey: Pubkey,
}

impl SolanaClient {
    pub fn new(
        endpoint: &str,
        mint_pubkey: Pubkey,
        authority_key: Keypair,
        commitment_config: CommitmentConfig,
    ) -> SolanaClient {
        let rpc_client = RpcClient::new_with_commitment(endpoint, CommitmentConfig::confirmed());
        SolanaClient {
            rpc_client,
            commitment_config,
            authority_key,
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
        from_height: u64,
    ) -> Result<Vec<(Self::TxID, Self::Address, Self::Amount)>, Self::Error> {
        let token_pubkey =
            get_associated_token_address(&self.authority_key.pubkey(), &self.mint_pubkey);
        // fetch signatures of transactions involving this token account
        let signatures = self
            .rpc_client
            .get_signatures_for_address_with_config(
                &token_pubkey,
                GetConfirmedSignaturesForAddress2Config {
                    before: None,
                    until: None,
                    limit: None,
                    commitment: Some(self.commitment_config),
                },
            )
            .unwrap();
        println!("the number of signatures is: {}", signatures.len());

        let mut withdrawals = vec![];
        for signature_info in signatures.iter() {
            let signature = signature_info.signature.parse::<Signature>().unwrap();
            println!("analyzing signature: {}", signature);
            let res = parse_transaction(&self.rpc_client, &signature, self.mint_pubkey);
            if let Ok(tpl_token_txs) = res {
                for tx in tpl_token_txs.iter() {
                    withdrawals.push((signature, tx.source, tx.amount));
                }
            }
        }

        Ok(withdrawals)
    }
}

#[cfg(test)]
mod tests {
    use crate::solana::{
        check_spl_token, get_or_create_associated_token_account, get_token_balance, init_spl_token,
        wait_transaction_until_processed, DEFAULT_MINT_AMOUNT,
    };

    use super::*;

    const ENDPOINT_DEVNET: &str = "https://api.devnet.solana.com";
    const AIRDROP_LAMPORTS: u64 = 1_000_000_000;
    const TRANSFER_LAMPORTS: u64 = 1_000;

    // Afa4Jc8cGhyQc6v64sVw7qpUMiHDrTSc2umPwEdvAZ9M
    const AUTHORITY_KEY: &str =
        "5KDTRK1s2b2oaopXqi2gjSaHgUuzfuvYSwNAND7EdgravGJ44mG1bHynM4UxfWz8dQNQ8TcbtTBM3NKfp4v4vUAo";

    // 8NXzZrJTs8TQYPNamLttfdVAVF3d8nPjqQRkJfJkdmyy
    const MINT_KEY: &str =
        "BwNBH51VS47q9tBeeRicPjfKB5k4ys3UkyjRD9wxWDnhDGpESsTywH5SPtb3cYG9Ec3gbezNM3SsjGZGNHqdBdR";

    // dWC1R5jgKfjH79qv4jANoL1Q6FcKGQLYGzRAbqYoqtc
    const TARGET_KEY: &str =
        "4Sn5MvhpuAstWo25nAhnn3Y5sVXvz2na54J8rnrW58FLQiZLAJxgtwPL3mZdniG2NPbDPt5WMeizWNPEqSydAJwA";

    #[test]
    fn test_send_load_history() {
        let rpc_client =
            RpcClient::new_with_commitment(ENDPOINT_DEVNET, CommitmentConfig::confirmed());

        let authority_key = Keypair::from_base58_string(AUTHORITY_KEY);
        println!(
            "authority_key: {}, pubkey: {}",
            authority_key.to_base58_string(),
            authority_key.pubkey()
        );

        let mint_key = Keypair::from_base58_string(MINT_KEY);
        let mint_pubkey = mint_key.pubkey();
        println!(
            "mint_key: {}, pubkey: {}",
            mint_key.to_base58_string(),
            mint_key.pubkey()
        );

        let res = check_spl_token(&rpc_client, &mint_pubkey);
        if res.is_err() {
            let signature = init_spl_token(
                &rpc_client,
                &authority_key,
                &mint_key,
                8,
                DEFAULT_MINT_AMOUNT,
            )
            .unwrap();
            wait_transaction_until_processed(
                &rpc_client,
                &signature,
                CommitmentConfig::confirmed(),
            )
            .unwrap();
            // check the token balance of the mint account
            let balance =
                get_token_balance(&rpc_client, &mint_pubkey, &authority_key.pubkey()).unwrap();
            assert_eq!(balance, DEFAULT_MINT_AMOUNT);
        }

        // create target token account
        let target_key = Keypair::from_base58_string(TARGET_KEY);
        println!(
            "target key: {}, pubkey: {}",
            target_key.to_base58_string(),
            target_key.pubkey()
        );
        let target_pubkey = target_key.pubkey();

        let client = SolanaClient::new(
            ENDPOINT_DEVNET,
            mint_pubkey.clone(),
            authority_key,
            CommitmentConfig::confirmed(),
        );

        let signature = client.send_solana(&target_pubkey, 30_000_000).unwrap();
        wait_transaction_until_processed(&rpc_client, &signature, CommitmentConfig::confirmed())
            .unwrap();

        let (_, signature_opt) =
            get_or_create_associated_token_account(&rpc_client, &mint_pubkey, &target_key).unwrap();
        if let Some(signature) = signature_opt {
            wait_transaction_until_processed(
                &rpc_client,
                &signature,
                CommitmentConfig::confirmed(),
            )
            .unwrap();
        }

        // send with SolanaClient
        client.send(&target_pubkey, TRANSFER_LAMPORTS).unwrap();
        wait_transaction_until_processed(&rpc_client, &signature, CommitmentConfig::confirmed())
            .unwrap();

        let balance = get_token_balance(&rpc_client, &mint_pubkey, &target_pubkey).unwrap();
        assert!(balance >= TRANSFER_LAMPORTS);

        let withdrawals = client.load_unfinished_withdrawals(0).unwrap();
        println!("total {} withdrawal(s)", withdrawals.len());
        assert!(withdrawals.len() > 0);
        for (signature, pubkey, amount) in withdrawals.iter() {
            println!(
                "signature: {}, pubkey {}, amount {}",
                signature, pubkey, amount
            );
        }
    }
}