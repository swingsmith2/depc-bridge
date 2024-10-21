#[derive(Debug)]
pub enum Error {
    MissingRequiredField,
    CannotCreateMintInstructions,
    CannotGetLatestBlockHash,
    CannotGetBlockHeight,
    CannotSendTransaction,
    CannotMakeMintTransaction,
    CannotGetAccountData,
    CannotGetAccountBalance,
    CannotUnpackAccountData,
    CannotGetStatusForSignature,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "something is wrong")
    }
}