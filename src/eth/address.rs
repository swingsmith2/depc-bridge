use std::str::FromStr;

#[derive(Debug)]
pub enum Error {
    InvalidAddressString,
}

pub struct Address {
    data: Vec<u8>,
}

impl ToString for Address {
    fn to_string(&self) -> String {
        let mut res = "0x".to_owned();
        res.push_str(&hex::encode(&self.data));
        res
    }
}

impl FromStr for Address {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Error> {
        if let Ok(data) = hex::decode(s) {
            Ok(Address { data })
        } else {
            Err(Error::InvalidAddressString)
        }
    }
}