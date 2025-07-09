use crate::parser::Object;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error during file IO operations: {0}")]
    IO(#[from] std::io::Error),
    #[error("Parser error: {message}")]
    Parser { message: String },

    #[error("Unexpected object type. Expected = {expected}, got = {got:?}")]
    InvalidObjectType { expected: String, got: Object },

    #[error("Unexpected stream type. Expected = {expected}, got = {got}")]
    InvalidStreamType { expected: String, got: String },
    #[error("Unexpected stream filter name. Got = {0}")]
    InvalidStreamFilterName(String),
    #[error("Unexpected stream length")]
    InvalidStreamLength,
}
