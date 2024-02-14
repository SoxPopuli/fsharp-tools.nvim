use std::fmt::Display;

#[derive(Debug)]
pub(crate) enum Error {
    FileError(String),
    IOError,
    ParseError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileError(msg) => write!(f, "FileError: {msg}"),
            Self::IOError => write!(f, "IOError"),
            Self::ParseError(msg) => write!(f, "ParseError: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn to_string(&self) -> String {
        format!("{}", self)
    }
}
