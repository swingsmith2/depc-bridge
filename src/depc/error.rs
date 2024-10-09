use std::fmt;

#[derive(Debug)]
pub enum Error {
    RpcError,
    InvalidHex,
    InvalidScript,
    NotOPReturn,
    InvalidStringFromScript,
    NotErc20Address,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::RpcError => write!(f, "RPC service error"),
            Error::InvalidHex => write!(f, "the hex string is invalid"),
            Error::InvalidScript => write!(f, "the script is invalid"),
            Error::NotOPReturn => write!(f, "the script is not started with OP_RETURN"),
            Error::InvalidStringFromScript => write!(f, "the stored string from script is invalid"),
            Error::NotErc20Address => write!(f, "cannot decode erc20 address from stored string"),
        }
    }
}

impl std::error::Error for Error {}
