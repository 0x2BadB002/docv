use snafu::{OptionExt, ResultExt, Snafu};

use crate::types::{Dictionary, IndirectReference, Rectangle};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct PagesTreeNode {
    pub leaf_count: usize,
    pub kids: Vec<IndirectReference>,
    pub inheritable_attributes: InheritableAttributes,
}

impl PagesTreeNode {
    pub fn from_dictionary(
        dictionary: &Dictionary,
        inheritable_attributes: Option<InheritableAttributes>,
    ) -> Result<Self> {
        let kids = dictionary
            .get("Kids")
            .context(error::FieldNotFound { field: "Kids" })?
            .as_array()
            .of(|obj| obj.as_indirect_ref().cloned())
            .context(error::InvalidArray { field: "Kids" })?;

        let count = dictionary
            .get("Count")
            .context(error::FieldNotFound { field: "Count" })?
            .as_integer()
            .context(error::InvalidType { field: "Count" })?;

        let mut inheritable_attributes = inheritable_attributes.unwrap_or_default();
        inheritable_attributes.read(dictionary)?;

        Ok(Self {
            leaf_count: count,
            kids,
            inheritable_attributes,
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct InheritableAttributes {
    pub resources: Option<Dictionary>,
    pub media_box: Option<Rectangle>,
    pub crop_box: Option<Rectangle>,
    pub rotate: Option<u16>,
}

impl InheritableAttributes {
    fn read(&mut self, dictionary: &Dictionary) -> Result<()> {
        let resources = dictionary
            .get("Resources")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidType { field: "Resources" })?;

        let media_box = dictionary
            .get("MediaBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArray { field: "MediaBox" })?;

        let crop_box = dictionary
            .get("CropBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArray { field: "CropBox" })?;

        let rotate = dictionary
            .get("Rotate")
            .map(|object| object.as_integer())
            .transpose()
            .context(error::InvalidType { field: "Rotate" })?;

        if resources.is_some() {
            self.resources = resources;
        }

        if media_box.is_some() {
            self.media_box = media_box;
        }

        if crop_box.is_some() {
            self.crop_box = crop_box;
        }

        if rotate.is_some() {
            self.rotate = rotate;
        }

        Ok(())
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

        #[snafu(display("Invalid array data for field `{field}`"))]
        InvalidArray {
            field: &'static str,
            source: crate::types::array::Error,
        },

        #[snafu(display("Unexpected node type. Got = `{got}`. Expected `Page` or `Pages`]"))]
        UnexpectedNodeType { got: String },
    }
}
