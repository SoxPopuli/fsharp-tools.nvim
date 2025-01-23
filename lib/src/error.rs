use std::fmt::Display;

#[derive(Debug)]
pub(crate) enum Error {
    FileError(String),
    LockError(String),
    IOError(std::io::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileError(msg) => write!(f, "FileError: {msg}"),
            Self::LockError(msg) => write!(f, "LockError: {msg}"),
            Self::IOError(error) => write!(f, "IOError: {}", error),
        }
    }
}

impl Error {
    pub fn file_error<E>(err: E) -> Self
    where
        E: std::error::Error,
    {
        Self::FileError(err.to_string())
    }
}

impl std::error::Error for Error {}

impl From<Error> for mlua::Error {
    fn from(value: Error) -> Self {
        mlua::Error::external(value)
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IOError(value)
    }
}

pub(crate) trait ResultToLuaError {
    type Item;
    fn to_lua_error(self) -> Result<Self::Item, mlua::Error>;
}
impl<T> ResultToLuaError for Result<T, Error> {
    type Item = T;

    fn to_lua_error(self) -> Result<Self::Item, mlua::Error> {
        self.map_err(mlua::Error::external)
    }
}

pub(crate) trait OptionToLuaError {
    type Item;
    fn to_lua_error(self, msg: String) -> Result<Self::Item, mlua::Error>;
}
impl<T> OptionToLuaError for Option<T> {
    type Item = T;
    fn to_lua_error(self, msg: String) -> Result<Self::Item, mlua::Error> {
        self.ok_or(mlua::Error::RuntimeError(msg))
    }
}
