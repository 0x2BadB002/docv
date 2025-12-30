use snafu::{OptionExt, ResultExt, Snafu};

pub mod pages;
pub mod version;

use crate::{
    objects::Objects,
    structures::root::{pages::PagesTreeNode, version::Version},
    types::{IndirectReference, Object},
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
#[allow(dead_code)]
pub struct Root {
    pub version: Option<Version>,
    pub extensions: Option<Object>,
    pub pages: PagesTreeNode,
    pub page_labels: Option<Object>,
    pub names: Option<Object>,
    pub dests: Option<Object>,
    pub viewer_preferences: Option<Object>,
    pub page_layout: PageLayout,
    pub page_mode: PageMode,
    pub outlines: Option<IndirectReference>,
    pub threads: Option<IndirectReference>,
    pub open_action: Option<Object>,
    pub aa: Option<Object>,
    pub uri: Option<Object>,
    pub acro_form: Option<Object>,
    pub metadata: Option<IndirectReference>,
    pub struct_tree_root: Option<Object>,
    pub mark_info: Option<Object>,
    pub lang: Option<Object>,
    pub spider_info: Option<Object>,
    pub output_intents: Option<Object>,
    pub piece_info: Option<Object>,
    pub oc_properities: Option<Object>,
    pub perms: Option<Object>,
    pub legal: Option<Object>,
    pub requirements: Option<Object>,
    pub collection: Option<Object>,
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
        .context(error::FailedCreatePages)?;

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
            extensions: None,
            page_labels: None,
            names: None,
            dests: None,
            viewer_preferences: None,
            page_layout,
            page_mode,
            open_action: None,
            aa: None,
            uri: None,
            acro_form: None,
            struct_tree_root: None,
            mark_info: None,
            lang: None,
            spider_info: None,
            output_intents: None,
            piece_info: None,
            oc_properities: None,
            perms: None,
            legal: None,
            requirements: None,
            collection: None,
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

        #[snafu(display("Failed to instantate `Pages` struct"))]
        FailedCreatePages {
            source: crate::structures::root::pages::Error,
        },

        #[snafu(display("Unexpected value for `PageLayout`. Got = `{value}`"))]
        UnexpectedPageLayout { value: String },
    }
}
