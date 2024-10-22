use solana_client::rpc_client::RpcClient;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Account as TokenAccount;

use super::{Builder, Error, NewFromBuilder};

pub struct Querier {
    rpc_client: RpcClient,
    mint_pubkey: Pubkey,
}

impl NewFromBuilder for Querier {
    type T = Querier;

    fn new_from_builder(builder: Builder) -> Result<Self::T, Error> {
        let rpc_client = builder.create_rpc_client_from_url()?;
        if builder.mint_pubkey.is_none() {
            return Err(Error::MissingRequiredField);
        }
        let mint_pubkey = builder.mint_pubkey.unwrap();
        Ok(Querier {
            rpc_client,
            mint_pubkey,
        })
    }
}

impl Querier {
    pub fn get_token_balance(&self, wallet_address: &Pubkey) -> Result<u64, Error> {
        let associated_token_address =
            get_associated_token_address(wallet_address, &self.mint_pubkey);

        // Fetch the token account info
        let res = self.rpc_client.get_account_data(&associated_token_address);
        if res.is_err() {
            return Err(Error::CannotGetAccountData);
        }
        let account_data = res.unwrap();

        // Deserialize the token account data
        let res = TokenAccount::unpack(&account_data);
        if res.is_err() {
            return Err(Error::CannotUnpackAccountData);
        }
        let token_account = res.unwrap();
        Ok(token_account.amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}