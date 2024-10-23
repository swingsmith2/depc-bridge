use solana_sdk::{pubkey::Pubkey, signature::Signature};

pub struct TransactionInfo {
    pub(crate) signature: Signature,
    pub(crate) instruction: InstructionInfo,
}

pub struct InstructionInfo {
    pub(crate) amount: u64,
    pub(crate) authority: Pubkey,
    pub(crate) destination: Pubkey,
    pub(crate) source: Pubkey,
    pub(crate) owner: Pubkey,
}
