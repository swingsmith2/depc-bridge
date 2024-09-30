use serde::Deserialize;

#[derive(Deserialize)]
pub struct Block {
    pub hash: String,
    pub height: u32,
    pub miner: String,
    pub time: u64,
    pub tx: Vec<String>,
}
