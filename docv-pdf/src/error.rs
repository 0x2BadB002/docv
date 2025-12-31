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
