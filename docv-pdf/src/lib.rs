mod document;
mod objects;
mod pages;
mod parser;
mod structures;
mod types;

pub use document::Document;
pub use structures::root::pages::Page;

#[derive(Debug, snafu::Snafu)]
pub struct Error(error::Error);

mod error {
    use snafu::Snafu;

    use crate::{document, pages};

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Error while reading document"))]
        Document { source: document::Error },

        #[snafu(display("Error while reading document pages"))]
        Pages { source: pages::Error },
    }
}
