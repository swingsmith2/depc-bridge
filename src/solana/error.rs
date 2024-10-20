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
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "something is wrong")
    }
}