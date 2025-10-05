use chrono::{DateTime, FixedOffset};
use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    objects::Objects,
    types::{Array, Dictionary, IndirectReference, Object, Rectangle, Stream},
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Pages<'a> {
    root: PagesTreeNode,
    node: PagesTreeNode,
    leaf_iter: std::vec::IntoIter<IndirectReference>,

    objects: &'a Objects,
}

impl<'a> Pages<'a> {
    pub fn new(root: &Dictionary, objects: &'a Objects) -> Result<Self> {
        let tree = PagesTreeNode::from_dictionary(root)?;

        Ok(Self {
            root: tree.clone(),
            leaf_iter: tree.kids.clone().into_iter(),
            node: tree,
            objects,
        })
    }
}

impl<'a> std::iter::Iterator for Pages<'a> {
    type Item = Page;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }

    fn count(self) -> usize {
        self.root.leaf_count
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.node.leaf_count, Some(self.root.leaf_count))
    }
}

#[derive(Debug)]
pub struct Page {
    contents: Vec<Stream>,
    resources: Dictionary,
    user_unit: f64,
    rotate: u16,

    media_box: Rectangle,
    crop_box: Rectangle,
    bleed_box: Rectangle,
    trim_box: Rectangle,
    art_box: Rectangle,

    last_modified: Option<DateTime<FixedOffset>>,
    box_color_info: Option<Dictionary>,
    group: Option<Dictionary>,
    thumb: Option<Stream>,
    b: Option<Array>,
    dur: Option<f64>,
    trans: Option<Dictionary>,
    annots: Option<Array>,
    aa: Option<Dictionary>,
    metadata: Option<Stream>,
    piece_info: Option<Dictionary>,
    struct_parents: Option<usize>,
    id: Option<Vec<u8>>,
    pz: Option<f64>,
    separation_info: Option<Dictionary>,
    tabs: TabOrder,
    template_instantiated: Option<String>,
    pres_steps: Option<Dictionary>,
    vp: Option<Dictionary>,
}

#[derive(Debug, Default)]
enum TabOrder {
    Row,
    Column,
    Structure,
    #[default]
    None,
}

impl Page {
    fn from_dictionary(
        dictionary: &Dictionary,
        inheritable_attrs: &InheritableAttributes,
        objects: &mut Objects,
    ) -> Result<Self> {
        let contents = dictionary
            .get("Contents")
            .context(error::FieldNotFoundSnafu { field: "Contents" })?;

        let contents = match contents {
            Object::Stream(stream) => vec![stream.clone()],
            Object::Array(array) => array
                .iter()
                .map(|object| object.as_stream().cloned())
                .collect::<std::result::Result<Vec<_>, _>>()
                .context(error::InvalidTypeSnafu { field: "Contents" })?,
            Object::IndirectReference(object_ref) => {
                let object = objects
                    .get_object(object_ref)
                    .ok()
                    .context(error::FieldNotFoundSnafu { field: "Contents" })?;

                match object {
                    Object::Stream(stream) => vec![stream],
                    Object::Array(array) => array
                        .iter()
                        .map(|object| object.as_stream().cloned())
                        .collect::<std::result::Result<Vec<_>, _>>()
                        .context(error::InvalidTypeSnafu { field: "Contents" })?,
                    _ => todo!(),
                }
            }
            _ => {
                todo!()
            }
        };

        let resources = dictionary
            .get("Resources")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "Resources" })?
            .or_else(|| inheritable_attrs.resources.clone())
            .context(error::FieldNotFoundSnafu { field: "Resources" })?;

        let media_box = dictionary
            .get("MediaBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArraySnafu { field: "MediaBox" })?
            .or_else(|| inheritable_attrs.media_box.clone())
            .context(error::FieldNotFoundSnafu { field: "MediaBox" })?;

        let crop_box = dictionary
            .get("CropBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArraySnafu { field: "CropBox" })?
            .or_else(|| inheritable_attrs.crop_box.clone())
            .unwrap_or_else(|| media_box.clone());

        let bleed_box = dictionary
            .get("BleedBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArraySnafu { field: "BleedBox" })?
            .unwrap_or_else(|| crop_box.clone());

        let trim_box = dictionary
            .get("TrimBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArraySnafu { field: "TrimBox" })?
            .unwrap_or_else(|| crop_box.clone());

        let art_box = dictionary
            .get("ArtBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArraySnafu { field: "ArtBox" })?
            .unwrap_or_else(|| crop_box.clone());

        let rotate = dictionary
            .get("Rotate")
            .map(|object| object.as_integer())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "Rotate" })?
            .unwrap_or(0);

        let user_unit = dictionary
            .get("UserUnit")
            .map(|object| object.as_float())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "UserUnit" })?
            .unwrap_or(1.0);

        let last_modified = dictionary
            .get("LastModified")
            .map(|object| -> Result<DateTime<FixedOffset>> {
                Ok(object
                    .as_string()
                    .context(error::InvalidTypeSnafu {
                        field: "LastModified",
                    })?
                    .to_date()
                    .context(error::InvalidDateSnafu {
                        field: "LastModified",
                    })?)
            })
            .transpose()?;

        let box_color_info = dictionary
            .get("BoxColorInfo")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidTypeSnafu {
                field: "BoxColorInfo",
            })?;

        let group = dictionary
            .get("Group")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "Group" })?;

        let thumb = dictionary
            .get("Thumb")
            .map(|object| object.as_stream().cloned())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "Thumb" })?;

        let b = dictionary
            .get("B")
            .map(|object| object.as_array().generic())
            .transpose()
            .context(error::InvalidArraySnafu { field: "B" })?;

        let dur = dictionary
            .get("Dur")
            .map(|object| object.as_float())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "Dur" })?;

        let trans = dictionary
            .get("Trans")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "Trans" })?;

        let annots = dictionary
            .get("Annots")
            .map(|object| object.as_array().generic())
            .transpose()
            .context(error::InvalidArraySnafu { field: "Annots" })?;

        let aa = dictionary
            .get("AA")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "AA" })?;

        let metadata = dictionary
            .get("Metadata")
            .map(|object| object.as_stream().cloned())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "Metadata" })?;

        let piece_info = dictionary
            .get("PieceInfo")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "PieceInfo" })?;

        let struct_parents = dictionary
            .get("Dur")
            .map(|object| object.as_integer())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "Dur" })?;

        let id = dictionary
            .get("ID")
            .map(|object| object.as_string())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "ID" })?
            .map(|s| s.as_bytes().to_vec());

        let pz = dictionary
            .get("PZ")
            .map(|object| object.as_float())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "PZ" })?;

        let separation_info = dictionary
            .get("SeparationInfo")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidTypeSnafu {
                field: "SeparationInfo",
            })?;

        let tabs = dictionary
            .get("Tabs")
            .map(|object| object.as_name())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "Tabs" })?
            .map(|name| match name {
                "R" => TabOrder::Row,
                "C" => TabOrder::Column,
                "S" => TabOrder::Structure,
                _ => TabOrder::default(),
            })
            .unwrap_or_default();

        let template_instantiated = dictionary
            .get("TemplateInstantiated")
            .map(|object| object.as_name())
            .transpose()
            .context(error::InvalidTypeSnafu {
                field: "TemplateInstantiated",
            })?
            .map(|s| s.to_string());

        let pres_steps = dictionary
            .get("PresSteps")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "PresSteps" })?;

        let vp = dictionary
            .get("VP")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "VP" })?;

        Ok(Self {
            contents,
            resources,
            user_unit,
            rotate,

            media_box,
            crop_box,
            bleed_box,
            trim_box,
            art_box,

            last_modified,
            box_color_info,
            group,
            thumb,
            b,
            dur,
            annots,
            aa,
            metadata,
            piece_info,
            struct_parents,
            id,
            pz,
            separation_info,
            tabs,
            template_instantiated,
            pres_steps,
            vp,
            trans,
        })
    }
}

#[derive(Debug, Clone)]
struct PagesTreeNode {
    parent: Option<IndirectReference>,
    leaf_count: usize,
    kids: Vec<IndirectReference>,
}

impl PagesTreeNode {
    fn from_dictionary(dictionary: &Dictionary) -> Result<Self> {
        let kids = dictionary
            .get("Kids")
            .context(error::FieldNotFoundSnafu { field: "Kids" })?
            .as_array()
            .of(|obj| obj.as_indirect_ref().cloned())
            .context(error::InvalidArraySnafu { field: "Kids" })?;

        let count = dictionary
            .get("Count")
            .context(error::FieldNotFoundSnafu { field: "Count" })?
            .as_integer()
            .context(error::InvalidTypeSnafu { field: "Count" })?;

        let mut inheritable_attrs = InheritableAttributes::default();
        inheritable_attrs.read(dictionary)?;

        Ok(Self {
            parent: None,
            leaf_count: count,
            kids,
        })
    }
}

#[derive(Debug, Default, Clone)]
struct InheritableAttributes {
    resources: Option<Dictionary>,
    media_box: Option<Rectangle>,
    crop_box: Option<Rectangle>,
    rotate: Option<u16>,
}

impl InheritableAttributes {
    fn read(&mut self, dictionary: &Dictionary) -> Result<()> {
        self.resources = dictionary
            .get("Resources")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "Resources" })?;

        self.media_box = dictionary
            .get("MediaBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArraySnafu { field: "MediaBox" })?;

        self.crop_box = dictionary
            .get("CropBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArraySnafu { field: "CropBox" })?;

        self.rotate = dictionary
            .get("Rotate")
            .map(|object| object.as_integer())
            .transpose()
            .context(error::InvalidTypeSnafu { field: "Rotate" })?;

        Ok(())
    }
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Required field {field} not found"))]
        FieldNotFound { field: &'static str },

        #[snafu(display("Invalid object type for field {field}"))]
        InvalidType {
            field: &'static str,
            source: crate::types::object::Error,
        },

        #[snafu(display("Invalid array data for field {field}"))]
        InvalidArray {
            field: &'static str,
            source: crate::types::array::Error,
        },

        #[snafu(display("Invalid date data for field {field}"))]
        InvalidDate {
            field: &'static str,
            source: crate::types::StringError,
        },
    }
}
