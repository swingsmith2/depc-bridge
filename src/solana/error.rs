#[derive(Debug)]
pub enum Error {
    MissingRequiredField,
    ExtractMismatchedType,
    CannotCreateMintInstructions,
    CannotGetLatestBlockHash,
    CannotGetBlockHeight,
    CannotSendTransaction,
    CannotMakeMintTransaction,
    CannotGetAccountData,
    CannotGetAccountBalance,
    CannotUnpackAccountData,
    CannotGetStatusForSignature,
    CannotGetTransactionInfo,
    CannotParsePubkeyFromString,
    CannotGetAssociatedAccount,
    CannotCreateAssociatedAccount,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "something is wrong")
    }
}
