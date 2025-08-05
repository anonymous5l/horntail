use std::char::DecodeUtf16Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::ErrorKind;
use std::{error, io};

pub enum Error {
    IO(io::Error),
    BrokenFile,
    InvalidVersion,
    UnexpectedData(String),
    InvalidCharacter,
    InvalidCipher,
    InvalidDataType,
    InvalidArgument,
    Unexpected(Box<dyn error::Error + Send + Sync>),
}

pub(crate) type Result<T> = std::result::Result<T, Error>;

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(e) => write!(f, "io: {e}"),
            Error::BrokenFile => write!(f, "broken file"),
            Error::InvalidVersion => write!(f, "invalid version"),
            Error::UnexpectedData(e) => write!(f, "unexpected data: {e}"),
            Error::InvalidCharacter => write!(f, "invalid character"),
            Error::InvalidCipher => write!(f, "invalid cipher"),
            Error::InvalidDataType => write!(f, "invalid data type"),
            Error::InvalidArgument => write!(f, "invalid argument"),
            Error::Unexpected(e) => write!(f, "unexpected: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::IO(value)
    }
}

impl From<DecodeUtf16Error> for Error {
    fn from(_: DecodeUtf16Error) -> Self {
        Self::InvalidCharacter
    }
}

#[cold]
pub(crate) fn io_err_invalid_input() -> Error {
    Error::IO(io::Error::new(ErrorKind::InvalidInput, "invalid path"))
}
