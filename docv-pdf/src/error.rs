use crate::parser::Object;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Error during file IO operations: {0}")]
    IO(#[from] std::io::Error),
    #[error("Error during str conversion operation: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Parser error: {message}")]
    Parser { message: String },

    #[error("Unexpected object type. Expected = {expected}, got = {got:?}")]
    InvalidObjectType { expected: String, got: Object },

    #[error("Encountered unknown dictionary field. Got = {0}")]
    UnknownDictionaryField(String),

    #[error("Unexpected stream type. Expected = {expected}, got = {got}")]
    InvalidStreamType { expected: String, got: String },
    #[error("Unexpected stream filter name. Got = {0}")]
    InvalidStreamFilterName(String),
    #[error("Unexpected stream length")]
    InvalidStreamLength,

    #[error("In provided xref no size was specified")]
    NoXrefSizeProvided,
    #[error("In provided xref ID array size was wrong. Expected = 2, got = {0}")]
    InvalidXrefIDSize(usize),
    #[error("In provided xref no root object was specified")]
    NoRootObjectProvided,
    #[error("In provided xref stream was specified invalid entry type")]
    UnexpectedXrefStreamEntryType(u64),
    #[error("In provided xref stream was specified invalid w array")]
    InvalidXrefStreamWSize(usize),
}
