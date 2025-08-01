use std::fmt::{Debug, Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    IO(std::io::Error),
    Horntail(horntail::Error),
    InvalidStructure,
    InvalidPackPaths,
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(e) => write!(f, "io: {e}"),
            Error::Horntail(e) => write!(f, "horntail: {e}"),
            Error::InvalidStructure => f.write_str("invalid structure"),
            Error::InvalidPackPaths => f.write_str("invalid pack paths"),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl std::error::Error for Error {}

impl From<horntail::Error> for Error {
    fn from(value: horntail::Error) -> Self {
        Error::Horntail(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IO(value)
    }
}
