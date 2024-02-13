use std::{
    fmt::{Debug, Display, Pointer},
};

#[derive(Debug)]
pub(crate) enum Error<'a> {
    FileError { path: &'a str },
    IOError,
    ParseError(String)
}

impl<'a> Display for Error<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileError { path } => write!(f, "FileError: {}", path),
            Self::IOError => write!(f, "IOError"),
            Self::ParseError(msg) => write!(f, "ParseError: {msg}"),
        }
    }
}

impl<'a> std::error::Error for Error<'a> {}

impl<'a> Error<'a> {
    pub fn to_string(&self) -> String {
        format!("{}", self)
    }
}
