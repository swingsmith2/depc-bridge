use std::{thread::sleep, time::Duration};

use serde_json::Value;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    account::ReadableAccount,
    commitment_config::CommitmentConfig,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction,
    transaction::Transaction,
};
use solana_transaction_status::{
    parse_instruction::ParsedInstruction, EncodedTransaction, UiInstruction, UiMessage,
    UiParsedInstruction, UiTransaction, UiTransactionEncoding,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::{
    instruction::{initialize_mint, mint_to, transfer},
    state::{Account as TokenAccount, Mint},
};

use super::Error;

pub const DEFAULT_LOCAL_ENDPOINT: &str = "https://api.devnet.solana.com";
pub const DEFAULT_MINT_AMOUNT: u64 = 83_000_000 * 10u64.pow(8);

pub fn check_spl_token(rpc_client: &RpcClient, mint_pubkey: &Pubkey) -> Result<u64, Error> {
    let res = rpc_client.get_account(&mint_pubkey);
    if let Err(e) = res {
        return Err(Error::InvalidMintAddress(mint_pubkey.to_string()));
    }
    let account = res.unwrap();
    if let Ok(mint) = Mint::unpack(account.data()) {
        return Ok(mint.supply);
    }
    Err(Error::InvalidMintAddress(mint_pubkey.to_string()))
}

pub fn init_spl_token(
    rpc_client: &RpcClient,
    authority_key: &Keypair,
    mint_key: &Keypair,
    decimals: u8,
    amount_to_mint: u64,
) -> Result<Signature, Error> {
    // Create a new keypair for the token mint account
    let authority_pubkey = authority_key.pubkey();
    let mint_pubkey = mint_key.pubkey();

    // Create the mint account
    let rent_exemption = rpc_client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .unwrap();
    let create_mint_account_instruction = system_instruction::create_account(
        &authority_pubkey,
        &mint_pubkey,
        rent_exemption,
        Mint::LEN as u64,
        &spl_token::id(),
    );

    // Initialize the mint
    let initialize_mint_instruction = initialize_mint(
        &spl_token::id(),
        &mint_pubkey,
        &authority_pubkey,
        Some(&authority_pubkey),
        decimals,
    )
    .unwrap();

    // Create associated token account for the payer
    let create_token_account_instruction =
        spl_associated_token_account::instruction::create_associated_token_account(
            &authority_pubkey,
            &authority_pubkey,
            &mint_pubkey,
            &spl_token::id(),
        );

    let account_pubkey = get_associated_token_address(&authority_pubkey, &mint_pubkey);

    // Mint some tokens to the associated token account
    let mint_to_instruction = mint_to(
        &spl_token::id(),
        &mint_pubkey,
        &account_pubkey,
        &authority_pubkey,
        &[],
        amount_to_mint,
    )
    .unwrap();

    // Build the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[
            create_mint_account_instruction,
            initialize_mint_instruction,
            create_token_account_instruction,
            mint_to_instruction,
        ],
        Some(&authority_pubkey),
        &[&authority_key, &mint_key],
        rpc_client.get_latest_blockhash().unwrap(),
    );

    // Send and confirm the transaction
    let signature = rpc_client
        .send_and_confirm_transaction(&transaction)
        .unwrap();

    Ok(signature)
}

pub fn get_token_balance(
    rpc_client: &RpcClient,
    mint_pubkey: &Pubkey,
    pubkey: &Pubkey,
) -> Result<u64, Error> {
    let associated_token_address = get_associated_token_address(&pubkey, &mint_pubkey);

    // Fetch the token account info
    let res = rpc_client.get_account_data(&associated_token_address);
    if res.is_err() {
        println!("get account data is failed, reason: {}", res.err().unwrap());
        return Err(Error::CannotGetAccountData(mint_pubkey.to_string()));
    }
    let account_data = res.unwrap();

    // Deserialize the token account data
    let res = TokenAccount::unpack(&account_data);
    if res.is_err() {
        return Err(Error::CannotUnpackAccountData(mint_pubkey.to_string()));
    }
    let token_account = res.unwrap();
    Ok(token_account.amount)
}

pub fn wait_transaction_until_processed(
    rpc_client: &RpcClient,
    signature: &Signature,
    commitment: CommitmentConfig,
) -> Result<(), Error> {
    println!("waiting signature {}...", signature);
    loop {
        let res = match rpc_client.get_signature_status_with_commitment(&signature, commitment) {
            Ok(s) => {
                if s.is_some() {
                    // ok, the tx is processed
                    println!("the tx {} is processed", signature);
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(e) => {
                println!("cannot get status for signature, reason: {}", e);
                return Err(Error::CannotGetStatusForSignature(signature.to_string()));
            }
        };
        if res.is_ok() {
            let succ = res.unwrap();
            if succ {
                break;
            } else {
                sleep(Duration::from_secs(1));
            }
        } else {
            return res.expect_err("this should be an error");
        }
    }
    Ok(())
}

pub fn create_associated_token_account_and_send(
    rpc_client: &RpcClient,
    mint_pubkey: &Pubkey,
    owner_key: &Keypair,
) -> Result<Signature, Error> {
    // we need to create th token account
    let instruction = create_associated_token_account(
        &owner_key.pubkey(),
        &owner_key.pubkey(),
        &mint_pubkey,
        &spl_token::id(),
    );
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&owner_key.pubkey()));
    let res = rpc_client.get_latest_blockhash();
    if let Err(e) = res {
        println!("cannot get latest blockhash, reason: {}", e);
        return Err(Error::CannotGetLatestBlockHash);
    }
    let recent_block_hash = res.unwrap();
    transaction.sign(&[&owner_key], recent_block_hash);
    let res = rpc_client.send_and_confirm_transaction(&transaction);
    if let Err(e) = res {
        println!("cannot send transaction, reason: {}", e);
        return Err(Error::CannotSendTransaction);
    }
    let signature = res.unwrap();
    Ok(signature)
}

pub fn get_or_create_associated_token_account(
    rpc_client: &RpcClient,
    mint_pubkey: &Pubkey,
    owner_key: &Keypair,
) -> Result<(Pubkey, Option<Signature>), Error> {
    let associated_token_address = get_associated_token_address(&owner_key.pubkey(), mint_pubkey);
    let mut signature = None;
    if rpc_client.get_account(&associated_token_address).is_err() {
        // we need to create th token account
        let res = create_associated_token_account_and_send(rpc_client, mint_pubkey, owner_key);
        if res.is_err() {
            return Err(Error::CannotCreateAssociatedAccount(
                owner_key.pubkey().to_string(),
            ));
        }
        signature = Some(res.unwrap());
    }
    Ok((associated_token_address, signature))
}

pub fn send_token(
    rpc_client: &RpcClient,
    mint_pubkey: &Pubkey,
    owner_key: &Keypair,
    target_pubkey: &Pubkey,
    amount: u64,
) -> Result<Signature, Error> {
    let source_token_pubkey = get_associated_token_address(&owner_key.pubkey(), mint_pubkey);
    let target_token_pubkey = get_associated_token_address(target_pubkey, mint_pubkey);

    let res = transfer(
        &spl_token::id(),
        &source_token_pubkey,
        &target_token_pubkey,
        &owner_key.pubkey(),
        &[&owner_key.pubkey()],
        amount,
    );
    if res.is_err() {
        return Err(Error::CannotMakeMintTransaction);
    }
    let instruction = res.unwrap();

    let res = rpc_client.get_latest_blockhash();
    if res.is_err() {
        return Err(Error::CannotGetLatestBlockHash);
    }
    let latest_block_hash = res.unwrap();
    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&owner_key.pubkey()));
    transaction.sign(&[&owner_key], latest_block_hash);

    let res = rpc_client.send_and_confirm_transaction(&transaction);
    if let Err(e) = res {
        println!("failed to send transaction, reason: {}", e);
        return Err(Error::CannotSendTransaction);
    }
    let signature = res.unwrap();
    Ok(signature)
}

mod parsing {
    use super::*;

    pub(super) fn parse_ui_message(ui_message: &UiMessage) -> Result<Vec<UiInstruction>, Error> {
        match ui_message {
            UiMessage::Parsed(message) => Ok(message.instructions.clone()),
            UiMessage::Raw(raw) => {
                println!("it's UiRawMessage: {:?}", raw.instructions);
                Err(Error::ExtractMismatchedType)
            }
        }
        // if let UiMessage::Parsed(message) = ui_message {
        //     Ok(message)
        // } else {
        //     println!("cannot extract UiMessage");
        //     Err(Error::ExtractMismatchedType)
        // }
    }

    pub(super) fn parse_ui_instruction(
        ui_instruction: &UiInstruction,
    ) -> Result<&UiParsedInstruction, Error> {
        if let UiInstruction::Parsed(instruction) = ui_instruction {
            Ok(instruction)
        } else {
            println!("cannot extract UiInstruction");
            Err(Error::ExtractMismatchedType)
        }
    }

    pub(super) fn parse_instruction_from_ui_parsed_instruction(
        instruction: &UiParsedInstruction,
    ) -> Result<&ParsedInstruction, Error> {
        if let UiParsedInstruction::Parsed(instruction) = instruction {
            Ok(instruction)
        } else {
            println!("cannot extract UiParsedInstruction");
            Err(Error::ExtractMismatchedType)
        }
    }
}

#[cfg(test)]
mod tests {
    use solana_sdk::commitment_config::CommitmentConfig;

    use super::*;

    const DEFAULT_AIRDROP_AMOUNT: u64 = 1_000_000_000;

    #[test]
    fn test_init_spl_token_and_mint_and_send() {
        let rpc_client =
            RpcClient::new_with_commitment(DEFAULT_LOCAL_ENDPOINT, CommitmentConfig::confirmed());
        let authority_key = Keypair::new();
        let mint_key = Keypair::new();
        let mint_pubkey = mint_key.pubkey();

        let signature = rpc_client
            .request_airdrop(&authority_key.pubkey(), DEFAULT_AIRDROP_AMOUNT)
            .unwrap();
        wait_transaction_until_processed(&rpc_client, &signature, CommitmentConfig::confirmed())
            .unwrap();

        let signature = init_spl_token(
            &rpc_client,
            &authority_key,
            &mint_key,
            8,
            DEFAULT_MINT_AMOUNT,
        )
        .unwrap();
        wait_transaction_until_processed(&rpc_client, &signature, CommitmentConfig::confirmed())
            .unwrap();

        // check the token balance of the mint account
        let balance =
            get_token_balance(&rpc_client, &mint_pubkey, &authority_key.pubkey()).unwrap();
        assert_eq!(balance, DEFAULT_MINT_AMOUNT);

        // create target token account
        let target_key = Keypair::new();
        let target_pubkey = target_key.pubkey();

        // don't forget the airdropping, else you don't have enough money to pay the fee
        let signature = rpc_client
            .request_airdrop(&target_pubkey, DEFAULT_AIRDROP_AMOUNT)
            .unwrap();
        wait_transaction_until_processed(&rpc_client, &signature, CommitmentConfig::confirmed())
            .unwrap();

        let (_, signature_opt) =
            get_or_create_associated_token_account(&rpc_client, &mint_pubkey, &target_key).unwrap();
        wait_transaction_until_processed(
            &rpc_client,
            &signature_opt.unwrap(),
            CommitmentConfig::confirmed(),
        )
        .unwrap();

        let signature = send_token(
            &rpc_client,
            &mint_pubkey,
            &authority_key,
            &target_pubkey,
            100,
        )
        .unwrap();
        wait_transaction_until_processed(&rpc_client, &signature, CommitmentConfig::confirmed())
            .unwrap();

        let balance = get_token_balance(&rpc_client, &mint_pubkey, &target_pubkey).unwrap();
        assert_eq!(balance, 100);
    }
}
