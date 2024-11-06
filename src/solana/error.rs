#[derive(Debug)]
pub enum Error {
    MissingRequiredField(String),
    ExtractMismatchedType,
    InvalidMintAddress(String),
    CannotCreateMintInstructions,
    CannotGetLatestBlockHash,
    CannotGetBlockHeight,
    CannotSendTransaction,
    CannotMakeMintTransaction,
    CannotGetAccountData(String),
    CannotGetAccountBalance(String),
    CannotUnpackAccountData(String),
    CannotGetStatusForSignature(String),
    CannotGetTransactionInfo(String),
    CannotParseTransactionInfo(String),
    CannotParsePubkeyFromString(String),
    CannotGetAssociatedAccount(String),
    CannotCreateAssociatedAccount(String),
    InvalidTransaction(String),
    CannotFetchTransaction(String),
    NotARelatedTransactionOfAuthority(String),
    MoreThanOneRelatedInstructionsFoundFrom1Transaction(String),
    CannotGetSignaturesForAddress(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRequiredField(field) => write!(f, "missing required field: {}", field),
            Self::ExtractMismatchedType => write!(f, "extract mismatched type"),
            Self::InvalidMintAddress(pubkey) => {
                write!(f, "the mint address is invalid: {}", pubkey)
            }
            Self::CannotCreateMintInstructions => write!(f, "cannot create mint instruction"),
            Self::CannotGetLatestBlockHash => write!(f, "cannot get latest block hash"),
            Self::CannotGetBlockHeight => write!(f, "cannot get block height"),
            Self::CannotSendTransaction => write!(f, "cannot send transaction"),
            Self::CannotMakeMintTransaction => write!(f, "cannot make mint transaction"),
            Self::CannotGetAccountData(pubkey) => write!(f, "cannot get account data: {}", pubkey),
            Self::CannotGetAccountBalance(pubkey) => {
                write!(f, "cannot get account balance: {}", pubkey)
            }
            Self::CannotUnpackAccountData(pubkey) => {
                write!(f, "cannot unpack account data: {}", pubkey)
            }
            Self::CannotGetStatusForSignature(signature) => {
                write!(f, "cannot get status for signature: {}", signature)
            }
            Self::CannotGetTransactionInfo(signature) => {
                write!(f, "cannot get transaction info: {}", signature)
            }
            Self::CannotParseTransactionInfo(signature) => {
                write!(f, "cannot parse transaction info: {}", signature)
            }
            Self::CannotParsePubkeyFromString(pubkey) => {
                write!(f, "cannot parse public-key from string: {}", pubkey)
            }
            Self::CannotGetAssociatedAccount(pubkey) => {
                write!(f, "cannot get associated account: {}", pubkey)
            }
            Self::CannotCreateAssociatedAccount(pubkey) => {
                write!(f, "cannot create associated account: {}", pubkey)
            }
            Self::InvalidTransaction(signature) => write!(f, "invalid transaction: {}", signature),
            Self::CannotFetchTransaction(signature) => {
                write!(f, "cannot fetch transaction: {}", signature)
            }
            Self::NotARelatedTransactionOfAuthority(signature) => write!(
                f,
                "the transaction (signature = {}) is not related to authority",
                signature
            ),
            Self::MoreThanOneRelatedInstructionsFoundFrom1Transaction(signature) => write!(
                f,
                "more than 1 related instructions found from the transaction {}",
                signature
            ),
            Self::CannotGetSignaturesForAddress(address) => {
                write!(f, "cannot get signatures for address: {}", address)
            }
        }
    }
}
