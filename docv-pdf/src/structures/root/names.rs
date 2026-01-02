use snafu::Snafu;

use crate::types::{Dictionary, Object};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Names {
    dests: Option<Object>,
    ap: Option<Object>,
    javascript: Option<Object>,
    pages: Option<Object>,
    templates: Option<Object>,
    ids: Option<Object>,
    urls: Option<Object>,
    embedded_files: Option<Object>,
    alternate_presentations: Option<Object>,
    renditions: Option<Object>,
}

impl Names {
    pub fn from_dictionary(dictionary: &Dictionary) -> Result<Self> {
        let dests = dictionary.get("Dests").cloned();
        let ap = dictionary.get("AP").cloned();
        let javascript = dictionary.get("Javascript").cloned();
        let pages = dictionary.get("Pages").cloned();
        let templates = dictionary.get("Templates").cloned();
        let ids = dictionary.get("IDS").cloned();
        let urls = dictionary.get("URLS").cloned();
        let embedded_files = dictionary.get("EmbeddedFiles").cloned();
        let alternate_presentations = dictionary.get("AlternatePresentations").cloned();
        let renditions = dictionary.get("Renditions").cloned();

        Ok(Names {
            dests,
            ap,
            javascript,
            pages,
            templates,
            ids,
            urls,
            embedded_files,
            alternate_presentations,
            renditions,
        })
    }
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Required field `{field}` not found"))]
        FieldNotFound { field: &'static str },

        #[snafu(display("Invalid object type for field `{field}`"))]
        InvalidType {
            field: &'static str,
            source: crate::types::object::Error,
        },
    }
}
