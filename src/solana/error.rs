pub enum Error {
    MissingUrl,
    MissingPayer,
    MissingContractAddress,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "something is wrong")
    }
}