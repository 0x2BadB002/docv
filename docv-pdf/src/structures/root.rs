use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    objects::Objects,
    structures::root::{names::Names, pages_tree::PagesTreeNode, version::Version},
    types::{IndirectReference, Object},
};

pub mod names;
pub mod pages_tree;
pub mod version;

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Root {
    pub version: Option<Version>,
    pub _extensions: Option<Object>,
    pub pages: PagesTreeNode,
    pub _page_labels: Option<Object>,
    pub names: Option<Names>,
    pub _dests: Option<Object>,
    pub _viewer_preferences: Option<Object>,
    pub page_layout: PageLayout,
    pub page_mode: PageMode,
    pub outlines: Option<IndirectReference>,
    pub threads: Option<IndirectReference>,
    pub _open_action: Option<Object>,
    pub _aa: Option<Object>,
    pub _uri: Option<Object>,
    pub _acro_form: Option<Object>,
    pub metadata: Option<IndirectReference>,
    pub _struct_tree_root: Option<Object>,
    pub _mark_info: Option<Object>,
    pub _lang: Option<Object>,
    pub _spider_info: Option<Object>,
    pub _output_intents: Option<Object>,
    pub _piece_info: Option<Object>,
    pub _oc_properities: Option<Object>,
    pub _perms: Option<Object>,
    pub _legal: Option<Object>,
    pub _requirements: Option<Object>,
    pub _collection: Option<Object>,
    pub needs_rendering: Option<bool>,
}

#[derive(Debug, Default)]
pub enum PageLayout {
    #[default]
    SinglePage,
    OneColumn,
    TwoColumnLeft,
    TwoColumnRight,
    TwoPageLeft,
    TwoPageRight,
}

#[derive(Debug, Default)]
pub enum PageMode {
    #[default]
    UseNone,
    UseOutlines,
    UseThumbs,
    FullScreen,
    UseOC,
    UseAttachments,
}

impl Root {
    pub fn from_object(object: Object, objects: &mut Objects) -> Result<Self> {
        let dictionary = object.as_dictionary().context(error::InvalidObject)?;

        let version = dictionary
            .get("Version")
            .map(Version::from_object)
            .transpose()
            .context(error::InvalidVersion)?;

        let pages = PagesTreeNode::from_dictionary(
            dictionary
                .get("Pages")
                .context(error::PagesNotFound)?
                .direct(objects)
                .as_dictionary()
                .context(error::InvalidType)?,
            None,
        )
        .context(error::InvalidPages)?;

        let names = dictionary
            .get("Names")
            .map(|object| -> Result<Names> {
                let object = object.direct(objects);
                let object = object.as_dictionary().context(error::InvalidType)?;
                Ok(Names::from_dictionary(object).context(error::InvalidNamesDictionary)?)
            })
            .transpose()?;

        let outlines = dictionary
            .get("Outlines")
            .map(|object| object.as_indirect_ref().cloned())
            .transpose()
            .context(error::InvalidType)?;

        let threads = dictionary
            .get("Threads")
            .map(|object| object.as_indirect_ref().cloned())
            .transpose()
            .context(error::InvalidType)?;

        let metadata = dictionary
            .get("Metadata")
            .map(|object| object.as_indirect_ref().cloned())
            .transpose()
            .context(error::InvalidType)?;

        let needs_rendering = dictionary
            .get("NeedsRendering")
            .map(|object| object.as_bool())
            .transpose()
            .context(error::InvalidType)?;

        let page_layout = dictionary
            .get("PageLayout")
            .map(|obj| {
                use PageLayout::*;

                let name = obj.as_name().context(error::InvalidType)?;
                match name {
                    "SinglePage" => Ok(SinglePage),
                    "OneColumn" => Ok(OneColumn),
                    "TwoColumnLeft" => Ok(TwoColumnLeft),
                    "TwoColumnRight" => Ok(TwoColumnRight),
                    "TwoPageLeft" => Ok(TwoPageLeft),
                    "TwoPageRight" => Ok(TwoPageRight),
                    _ => Err(error::Error::UnexpectedPageLayout {
                        value: name.to_string(),
                    }),
                }
            })
            .transpose()?
            .unwrap_or_default();

        let page_mode = dictionary
            .get("PageMode")
            .map(|obj| {
                use PageMode::*;

                let name = obj.as_name().context(error::InvalidType)?;
                match name {
                    "UseNone" => Ok(UseNone),
                    "UseOutlines" => Ok(UseOutlines),
                    "UseThumbs" => Ok(UseThumbs),
                    "FullScreen" => Ok(FullScreen),
                    "UseOC" => Ok(UseOC),
                    "UseAttachments" => Ok(UseAttachments),
                    _ => Err(error::Error::UnexpectedPageLayout {
                        value: name.to_string(),
                    }),
                }
            })
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            version,
            pages,
            outlines,
            threads,
            metadata,
            needs_rendering,
            names,
            page_layout,
            page_mode,
            _extensions: None,
            _page_labels: None,
            _dests: None,
            _viewer_preferences: None,
            _open_action: None,
            _aa: None,
            _uri: None,
            _acro_form: None,
            _struct_tree_root: None,
            _mark_info: None,
            _lang: None,
            _spider_info: None,
            _output_intents: None,
            _piece_info: None,
            _oc_properities: None,
            _perms: None,
            _legal: None,
            _requirements: None,
            _collection: None,
        })
    }
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Invalid object"))]
        InvalidObject { source: crate::types::object::Error },

        #[snafu(display("Invalid version field"))]
        InvalidVersion {
            source: crate::structures::root::version::Error,
        },

        #[snafu(display("Invalid field type"))]
        InvalidType { source: crate::types::object::Error },

        #[snafu(display("`Pages` field not found"))]
        PagesNotFound,

        #[snafu(display("Failed to instantate `PagesTreeNode` struct"))]
        InvalidPages {
            source: crate::structures::root::pages_tree::Error,
        },

        #[snafu(display("Invalid `Names` struct"))]
        InvalidNamesDictionary {
            source: crate::structures::root::names::Error,
        },

        #[snafu(display("Unexpected value for `PageLayout`. Got = `{value}`"))]
        UnexpectedPageLayout { value: String },
    }
}
