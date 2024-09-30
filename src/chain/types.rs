use serde::Deserialize;

#[derive(Deserialize)]
pub struct Block {
    pub hash: String,
    pub height: u32,
    pub miner: String,
    pub time: u64,
    pub tx: Vec<String>,
}

#[derive(Deserialize)]
pub struct ScriptPubKey {
    pub asm: String,
    pub hex: String,
    #[serde(rename = "reqSigs")]
    pub req_sigs: Option<u32>,
    pub r#type: String,
    pub addresses: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct ScriptSig {
    pub asm: String,
    pub hex: String,
}

#[derive(Deserialize)]
pub struct In {
    pub coinbase: Option<String>,
    pub txid: Option<String>,
    pub vout: Option<u32>,
    pub value: Option<String>,
    #[serde(rename = "scriptSig")]
    pub script_sig: Option<ScriptSig>,
    pub txinwitness: Option<Vec<String>>,
    pub sequence: u64,
}

#[derive(Deserialize)]
pub struct Out {
    pub value64: u64,
    pub value: f64,
    pub n: u32,
    #[serde(rename = "scriptPubKey")]
    pub script_pubkey: ScriptPubKey,
}

#[derive(Deserialize)]
pub struct Transaction {
    pub txid: String,
    pub vin: Vec<In>,
    pub vout: Vec<Out>,
}
