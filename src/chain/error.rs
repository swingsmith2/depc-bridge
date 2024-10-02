#[derive(Debug)]
pub enum Error {
    General,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::General => {
                write!(f, "General error")
            }
        }
    }
}

impl std::error::Error for Error {}
