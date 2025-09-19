use chrono::{DateTime, FixedOffset};
use snafu::{OptionExt, ResultExt, Snafu};

use crate::types::{Array, Dictionary, IndirectReference, Object, Stream};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct PagesTreeRoot {
    kids: Vec<IndirectReference>,
    pub count: usize,
}

pub struct PagesTreeNode {
    parent: IndirectReference,
    kids: Vec<IndirectReference>,
    count: usize,
}

pub struct Page {
    last_modified: Option<DateTime<FixedOffset>>,
    resources: Option<Dictionary>,
    media_box: Option<Array>,
    crop_box: Option<Array>,
    bleed_box: Option<Array>,
    trim_box: Option<Array>,
    art_box: Option<Array>,
    box_color_info: Option<Dictionary>,
    contents: Vec<Stream>,
    rotate: Option<u16>,
    group: Option<Dictionary>,
    thumb: Option<Stream>,
    b: Option<Array>,
    dur: Option<f32>,
    trans: Option<Dictionary>,
    annots: Option<Array>,
    aa: Option<Dictionary>,
    metadata: Option<Stream>,
    piece_info: Option<Dictionary>,
    struct_parents: Option<usize>,
    id: Option<Vec<u8>>,
    pz: Option<f32>,
    separation_info: Option<Dictionary>,
    tabs: Option<String>,
    template_instantiated: Option<String>,
    pres_steps: Option<Dictionary>,
    user_unit: Option<f32>,
    vp: Option<Dictionary>,
}

struct InheritableAttributes {
    resources: Option<Dictionary>,
    media_box: Option<Array>,
    crop_box: Option<Array>,
    rotate: Option<u16>,
}

impl PagesTreeRoot {
    pub fn from_object(object: Object) -> Result<Self> {
        let dictionary = object.as_dictionary().context(error::InvalidTypeSnafu)?;

        let kids = dictionary
            .get("Kids")
            .context(error::FieldNotFoundSnafu)?
            .as_array()
            .context(error::InvalidTypeSnafu)?
            .as_vec()
            .iter()
            .map(|object| object.as_indirect_ref().cloned())
            .collect::<std::result::Result<Vec<_>, _>>()
            .context(error::InvalidTypeSnafu)?;

        let count = dictionary
            .get("Count")
            .context(error::FieldNotFoundSnafu)?
            .as_integer()
            .context(error::InvalidTypeSnafu)?;

        Ok(Self { kids, count })
    }
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Required field not found"))]
        FieldNotFound,

        #[snafu(display("Invalid object passed"))]
        InvalidType { source: crate::types::ObjectError },
    }
}
