use super::Transaction;

#[derive(Debug)]
pub enum ClientError {}

pub struct Client;

impl Client {
    pub fn send(transaction: &Transaction) -> Result<String, ClientError> {
        todo!("complete this method");
    }
}
