use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Mint;

use super::{Builder, Error, NewFromBuilder};

pub struct Deploy {
    rpc_client: RpcClient,
    authority_key: Keypair,
    mint_key: Keypair,
}

impl NewFromBuilder for Deploy {
    type T = Deploy;

    fn new_from_builder(builder: Builder) -> Result<Self::T, Error> {
        let rpc_client = builder.new_rpc_client()?;
        if builder.authority_key.is_none() {
            return Err(Error::MissingRequiredField);
        }
        let authority_key = builder.authority_key.unwrap();
        if builder.mint_key.is_none() {
            return Err(Error::MissingRequiredField);
        }
        let mint_key = builder.mint_key.unwrap();
        Ok(Deploy {
            rpc_client,
            authority_key,
            mint_key,
        })
    }
}

impl Deploy {
    pub fn deploy(&self) -> Result<Signature, Error> {
        let space = Mint::LEN;
        let rent = self
            .rpc_client
            .get_minimum_balance_for_rent_exemption(space)
            .expect("Failed to get rent exemption");

        // Create the mint account
        let create_mint_account_ix = system_instruction::create_account(
            &self.authority_key.pubkey(),
            &self.mint_key.pubkey(),
            rent,
            space as u64,
            &spl_token::id(),
        );

        // Initialize the mint
        // total supply should be 84,000,000
        let res = spl_token::instruction::initialize_mint(
            &spl_token::id(),
            &self.mint_key.pubkey(),
            &self.authority_key.pubkey(),
            None,
            8,
        );
        if res.is_err() {
            return Err(Error::CannotCreateMintInstructions);
        }
        let init_mint_ix = res.unwrap();

        let res = self.rpc_client.get_latest_blockhash();
        if res.is_err() {
            return Err(Error::CannotGetLatestBlockHash);
        }
        let block_hash = res.unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[create_mint_account_ix, init_mint_ix],
            Some(&self.authority_key.pubkey()),
            &[&self.authority_key, &self.mint_key],
            block_hash,
        );

        let res = self.rpc_client.send_and_confirm_transaction(&transaction);
        if let Err(e) = res {
            println!("failed to send and confirm transaction, reason: {}", e);
            return Err(Error::CannotSendTransaction);
        }
        let signature = res.unwrap();
        Ok(signature)
    }

    pub fn mint_to(&self, recipient: Pubkey, amount: u64) -> Result<Signature, Error> {
        // Get the recipient's associated token account (ATA)
        let recipient_ata = get_associated_token_address(&recipient, &self.mint_key.pubkey());

        // Create the recipient's ATA if it doesn't exist
        let create_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account(
                &self.authority_key.pubkey(),
                &recipient,
                &self.mint_key.pubkey(),
                &spl_token::id(),
            );

        let res = spl_token::instruction::mint_to(
            &spl_token::id(),
            &self.mint_key.pubkey(),
            &recipient_ata,
            &self.authority_key.pubkey(),
            &[&self.authority_key.pubkey()],
            amount,
        );
        if res.is_err() {
            return Err(Error::CannotSendTransaction);
        }
        let mint_to_ix = res.unwrap();

        let res = self.rpc_client.get_latest_blockhash();
        if res.is_err() {
            return Err(Error::CannotGetLatestBlockHash);
        }
        let block_hash = res.unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[create_ata_ix, mint_to_ix],
            Some(&self.authority_key.pubkey()),
            &[&self.authority_key],
            block_hash,
        );
        let res = self.rpc_client.send_and_confirm_transaction(&transaction);
        if res.is_err() {
            return Err(Error::CannotSendTransaction);
        }
        let signature = res.unwrap();
        Ok(signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deploy() {
        let deploy = Builder::new()
            .set_url_localhost()
            .set_random_mint_key()
            .set_random_authority_key()
            .build::<Deploy>()
            .unwrap();
        let signature = deploy.deploy().unwrap();
        println!("signature: {}", signature);
    }
}