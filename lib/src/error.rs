use std::fmt::Display;

#[derive(Debug)]
pub(crate) enum Error {
    FileError(String),
    IOError,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileError(msg) => write!(f, "FileError: {msg}"),
            Self::IOError => write!(f, "IOError"),
        }
    }
}

impl std::error::Error for Error {}

impl From<Error> for mlua::Error {
    fn from(value: Error) -> Self {
        mlua::Error::external(value)
    }
}

pub(crate) trait ResultToLuaError {
    type Item;
    fn to_lua_error(self) -> Result<Self::Item, mlua::Error>;
}
impl<T> ResultToLuaError for Result<T, Error> {
    type Item = T;

    fn to_lua_error(self) -> Result<Self::Item, mlua::Error> {
        self.map_err( mlua::Error::external )
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
