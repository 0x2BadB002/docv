mod document;
mod error;
mod objects;
mod pages;
mod parser;
mod structures;
mod types;

pub use document::Document;
pub use structures::root::pages::Page;

#[derive(Debug, snafu::Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;
