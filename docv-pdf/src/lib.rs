mod document;
mod objects;
mod parser;
mod structures;
mod types;

pub use document::Document;
pub use structures::root::pages::Page;

#[derive(Debug, snafu::Snafu)]
pub struct Error(error::Error);

mod error {
    use snafu::Snafu;

    use crate::{document, structures};

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Error while reading document"))]
        Document { source: document::Error },

        #[snafu(display("Error while reading document pages"))]
        Pages {
            source: structures::root::pages::Error,
        },
    }
}
