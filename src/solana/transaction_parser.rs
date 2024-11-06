use std::str::FromStr;

use solana_client::rpc_client::RpcClient;
use solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature;
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use solana_transaction_status::{
    EncodedTransaction, UiInstruction, UiMessage, UiParsedInstruction, UiTransactionEncoding,
};

pub struct TplTokenTransaction {
    pub(crate) source: Pubkey,
    pub(crate) destination: Pubkey,
    pub(crate) amount: u64,
}

use super::Error;

#[derive(serde::Serialize)]
pub struct TransactionDetail {
    pub signature: String,
    pub source: String,
    pub destination: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: u64,
    pub tx_type: String,
}

/// # Load a transaction by the signature through RPC service
///
/// * `rpc_client`: The RPC service connection is established by this client object
/// * `signature`: The signature represents the transaction from solana network
/// * `authority_pubkey`: The public-key of the authority, the source/destination
///
/// # Return
/// A set of TplTokenTransaction objects. A transaction might contains more than one
/// instructions, so there is an object list should be returned, if the list is empty
/// that means the transaction doesn't contain any tpl-token record.
pub fn parse_tpl_token_signature(
    rpc_client: &RpcClient,
    signature: &Signature,
    authority_pubkey: &Pubkey,
) -> Result<Vec<TplTokenTransaction>, Error> {
    let mut tpl_token_txs = vec![];
    let res = rpc_client.get_transaction(&signature, UiTransactionEncoding::JsonParsed);
    if let Err(e) = res {
        println!("failed to get transaction {}, reason: {}", signature, e);
        return Err(Error::CannotGetTransactionInfo(signature.to_string()));
    }
    let transaction_meta = res.unwrap();
    let transaction = &transaction_meta.transaction.transaction;
    if let EncodedTransaction::Json(transaction) = transaction {
        if let UiMessage::Parsed(message) = &transaction.message {
            for instruction in message.instructions.iter() {
                if let UiInstruction::Parsed(UiParsedInstruction::Parsed(instruction)) = instruction
                {
                    // we need to confirm the instruction type is 'transfer'
                    let ty = instruction.parsed["type"].as_str().unwrap();
                    if ty != "transfer" {
                        continue;
                    }
                    // check the program-id and ensure it is related to our mint program
                    let program_id = Pubkey::from_str(&instruction.program_id).unwrap();
                    if program_id == spl_token::id() {
                        // it's tpl-token
                        let info = &instruction.parsed["info"];
                        println!("spl-token info: {}", info.to_string());
                        // ensure the instruction related to the authority's spl-token
                        let source = Pubkey::from_str(&info["source"].as_str().unwrap()).unwrap();
                        let destination =
                            Pubkey::from_str(&info["destination"].as_str().unwrap()).unwrap();
                        if source == *authority_pubkey || destination == *authority_pubkey {
                            let amount = info["amount"].as_str().unwrap().parse().unwrap();
                            tpl_token_txs.push(TplTokenTransaction {
                                source,
                                destination,
                                amount,
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(tpl_token_txs)
}

pub fn parse_signatures_for_target(
    rpc_client: &RpcClient,
    signatures: Vec<RpcConfirmedTransactionStatusWithSignature>,
) -> Result<Vec<TransactionDetail>, Error>
{
    let mut parsed_transactions = vec![];
    for signature_info in signatures {
        let signature = Signature::from_str(&signature_info.signature).unwrap();
        let transaction_meta_res= rpc_client.get_transaction(&signature, UiTransactionEncoding::JsonParsed);

        if let Ok(transaction_meta) = transaction_meta_res {
            // Access fee from the transaction's meta field
            let fee = transaction_meta.transaction.meta.as_ref().map_or(0, |meta| meta.fee);

            let transaction = &transaction_meta.transaction.transaction;

            if let EncodedTransaction::Json(transaction) = transaction {
                if let UiMessage::Parsed(message) = &transaction.message {
                    for instruction in message.instructions.iter() {
                        if let UiInstruction::Parsed(UiParsedInstruction::Parsed(instruction)) =
                            instruction
                        {
                            let ty = instruction.parsed["type"].as_str().unwrap_or("");

                            if ty == "transfer" {
                                let program_id = Pubkey::from_str(&instruction.program_id).unwrap();

                                if program_id == solana_sdk::system_program::id() {
                                    // SOL transfer
                                    let source = instruction.parsed["info"]["source"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_string();
                                    let destination = instruction.parsed["info"]["destination"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_string();
                                    let amount = instruction.parsed["info"]["lamports"]
                                        .as_str()
                                        .unwrap_or("0")
                                        .parse::<u64>()
                                        .unwrap_or(0);

                                    parsed_transactions.push(TransactionDetail {
                                        signature: signature.to_string(),
                                        source,
                                        destination,
                                        amount,
                                        fee,
                                        timestamp: signature_info.block_time.unwrap_or(0) as u64,
                                        tx_type: "sol".to_string(),
                                    });
                                } else if program_id == spl_token::id() {
                                    // SPL Token transfer
                                    let info = &instruction.parsed["info"];
                                    let source = info["source"].as_str().unwrap_or_default().to_string();
                                    let destination = info["destination"].as_str().unwrap_or_default().to_string();
                                    let amount = info["amount"].as_str().unwrap_or("0").parse::<u64>().unwrap_or(0);

                                    parsed_transactions.push(TransactionDetail {
                                        signature: signature.to_string(),
                                        source,
                                        destination,
                                        amount,
                                        fee,
                                        timestamp: signature_info.block_time.unwrap_or(0) as u64,
                                        tx_type: "token".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(parsed_transactions)
}

pub fn parse_tpl_token_signature_for_target(
    rpc_client: &RpcClient,
    signature: &Signature,
    authority_pubkey: &Pubkey,
) -> Result<Vec<TplTokenTransaction>, Error> {
    let mut tpl_token_txs = vec![];
    let res = rpc_client.get_transaction(&signature, UiTransactionEncoding::JsonParsed);
    if let Err(e) = res {
        println!("failed to get transaction {}, reason: {}", signature, e);
        return Err(Error::CannotGetTransactionInfo(signature.to_string()));
    }
    let transaction_meta = res.unwrap();
    let transaction = &transaction_meta.transaction.transaction;
    if let EncodedTransaction::Json(transaction) = transaction {
        if let UiMessage::Parsed(message) = &transaction.message {
            for instruction in message.instructions.iter() {
                if let UiInstruction::Parsed(UiParsedInstruction::Parsed(instruction)) = instruction
                {
                    // we need to confirm the instruction type is 'transfer'
                    let ty = instruction.parsed["type"].as_str().unwrap();
                    if ty != "transfer" {
                        continue;
                    }
                    // check the program-id and ensure it is related to our mint program
                    let program_id = Pubkey::from_str(&instruction.program_id).unwrap();
                    if program_id == spl_token::id() {
                        // it's tpl-token
                        let info = &instruction.parsed["info"];
                        println!("spl-token info: {}", info.to_string());
                        // ensure the instruction related to the authority's spl-token
                        let source = Pubkey::from_str(&info["source"].as_str().unwrap()).unwrap();
                        let destination =
                            Pubkey::from_str(&info["destination"].as_str().unwrap()).unwrap();
                        if destination == *authority_pubkey {
                            let amount = info["amount"].as_str().unwrap().parse().unwrap();
                            tpl_token_txs.push(TplTokenTransaction {
                                source,
                                destination,
                                amount,
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(tpl_token_txs)
}

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use solana_sdk::{commitment_config::CommitmentConfig, signature::Keypair, signer::Signer};
    use spl_associated_token_account::get_associated_token_address;

    use super::*;

    const DEFAULT_LOCAL_ENDPOINT: &str = "https://api.devnet.solana.com";

    const TEST_SIGNATURE: &str =
        "25A1pSwLHvagx8FD3oyAGot1Kfp9keqFhdfGgDZq4s9xjkPc4h5R3P6ikf5ookcsKuZEJDcFShsa3JdgVXYbmgRx";

    // Afa4Jc8cGhyQc6v64sVw7qpUMiHDrTSc2umPwEdvAZ9M
    const AUTHORITY_KEY: &str =
        "5KDTRK1s2b2oaopXqi2gjSaHgUuzfuvYSwNAND7EdgravGJ44mG1bHynM4UxfWz8dQNQ8TcbtTBM3NKfp4v4vUAo";

    // 8NXzZrJTs8TQYPNamLttfdVAVF3d8nPjqQRkJfJkdmyy
    const MINT_KEY: &str =
        "BwNBH51VS47q9tBeeRicPjfKB5k4ys3UkyjRD9wxWDnhDGpESsTywH5SPtb3cYG9Ec3gbezNM3SsjGZGNHqdBdR";

    const SPL_TOKEN_SIGNATURE: &str =
        "58pf2apLq8Uti8b45jKedN9chbPveiW6PeMUTXBvZ2UwgHdhtCoRtRK3R97Jre27DDQD8adztXhTwV9yNvBhBymV";

    #[test]
    fn test_parse() {
        let authority_key = Keypair::from_base58_string(AUTHORITY_KEY);
        let mint_key = Keypair::from_base58_string(MINT_KEY);

        let rpc_client =
            RpcClient::new_with_commitment(DEFAULT_LOCAL_ENDPOINT, CommitmentConfig::confirmed());

        let associated_pubkey =
            get_associated_token_address(&authority_key.pubkey(), &mint_key.pubkey());
        println!("authority associated pubkey: {}", associated_pubkey);

        let signature = Signature::from_str(SPL_TOKEN_SIGNATURE).unwrap();
        let records =
            parse_tpl_token_signature(&rpc_client, &signature, &associated_pubkey).unwrap();
        assert!(!records.is_empty());
    }
}