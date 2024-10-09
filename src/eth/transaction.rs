use serde::Serialize;

use super::address::Address;

trait ToHexValue {
    fn to_hex_value(&self) -> String;
}

impl ToHexValue for u64 {
    fn to_hex_value(&self) -> String {
        let mut res = "0x".to_owned();
        res.push_str(&hex::encode(self.to_le_bytes()));
        res
    }
}

#[derive(Serialize)]
pub struct Transaction {
    pub from: String,
    pub gas: String,
    #[serde(rename = "maxFeePerGas")]
    max_fee_per_gas: String,
    #[serde(rename = "maxPriorityFeePerGas")]
    pub max_priority_fee_per_gas: String,
    pub input: String,
    pub nonce: String,
    pub to: String,
    pub value: String,
}

pub enum BuildError {
    NoFrom,
    NoTo,
    InvalidGas,
}

pub struct TransactionBuilder {
    from: Option<Address>,
    gas: u64,
    max_fee_per_gas: u64,
    max_priority_fee_per_gas: u64,
    input: u64,
    nonce: u64,
    to: Option<Address>,
    value: u64,
}

impl TransactionBuilder {
    pub fn new() -> TransactionBuilder {
        TransactionBuilder {
            from: None,
            gas: 0,
            max_fee_per_gas: 0,
            max_priority_fee_per_gas: 0,
            input: 0,
            nonce: 0,
            to: None,
            value: 0,
        }
    }

    pub fn build(self) -> Result<Transaction, BuildError> {
        if self.from.is_none() {
            return Err(BuildError::NoFrom);
        }
        if self.to.is_none() {
            return Err(BuildError::NoTo);
        }
        if self.gas == 0 {
            return Err(BuildError::InvalidGas);
        }
        Ok(Transaction {
            from: self.from.unwrap().to_string(),
            gas: self.gas.to_hex_value(),
            max_fee_per_gas: self.max_fee_per_gas.to_hex_value(),
            max_priority_fee_per_gas: self.max_priority_fee_per_gas.to_hex_value(),
            input: self.input.to_hex_value(),
            nonce: self.nonce.to_hex_value(),
            to: self.to.unwrap().to_string(),
            value: self.value.to_hex_value(),
        })
    }

    pub fn set_from(mut self, from: Address) -> TransactionBuilder {
        self.from = Some(from);
        self
    }

    pub fn set_to(mut self, to: Address) -> TransactionBuilder {
        self.to = Some(to);
        self
    }

    pub fn set_value(mut self, value: u64) -> TransactionBuilder {
        self.value = value;
        self
    }
}
