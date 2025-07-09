use crate::parser::Rule;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error during file IO operations: {0}")]
    IO(#[from] std::io::Error),
    #[error("Error during parsing to numeric types: {0}")]
    IntConv(#[from] std::num::ParseIntError),

    #[error("Error during convertion of bytes to UTF-8 string: {0}")]
    FromStringUTF8(#[from] std::string::FromUtf8Error),
    #[error("Error during convertion of bytes to UTF-8 str: {0}")]
    FromStrUTF8(#[from] std::str::Utf8Error),

    #[error("Unexpected token: {0}")]
    InvalidStartXref(String),
    #[error("Unexpected token: {0}")]
    InvalidXref(String),
    #[error("Unexpected token: {0}")]
    InvalidTrailer(String),
    #[error("Unexpected token: {0}")]
    InvalidObject(String),

    #[error("Parse error at {line}:{column}{reason}")]
    Grammar {
        line: usize,
        column: usize,
        reason: String,
    },
    #[error("Object {0} - expected rule not found. Expected: {1:?}")]
    InvalidTokenPassed(String, Rule),

    #[error("Too many entries in xref table. Expected: {0}, Got: {1}")]
    InvalidXrefTableSize(usize, usize),
    #[error("Wrong xref stream W size: {0}")]
    InvalidXrefStreamWSize(usize),
    #[error("Wrong xref ID array size: {0}")]
    InvalidXrefIDSize(usize),
    #[error("Unexpected Xref stream entry type: {0}")]
    UnexpectedXrefStreamEntryType(u64),

    #[error("Root object is not provided")]
    NoRootObjectProvided,
    #[error("Xref size is not provided")]
    NoXrefSizeProvided,

    #[error("Missing stream token {0}")]
    InvalidStreamDeclaration(String),
    #[error("Wrong stream type {0}")]
    InvalidStreamType(String),
    #[error("Stream length wasn't specified")]
    InvalidStreamLength,
    #[error("Unsupported filter type: {0}")]
    UnhandledFilterType(String),
    #[error("Error during stream decompression: {0}")]
    ZlibCompression(#[from] flate2::DecompressError),

    #[error("Unexpected name: {0}")]
    UnexpectedName(String),

    #[error("Unexpected dictionary field: {0}")]
    UnexpectedDictionaryField(String),

    #[error("Invalid string passed")]
    InvalidString,
    #[error("Invalid literal string: {0}")]
    InvalidLiteralString(String),
    #[error("Invalid character \"{character}\" hex string: {hex_string}")]
    InvalidHexString { character: char, hex_string: String },
}
