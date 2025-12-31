use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    objects::Objects,
    types::{Array, Dictionary, IndirectReference, Rectangle, Stream, string::Date},
};

#[derive(Debug, Snafu)]
#[snafu(source(from(error::Error, Box::new)))]
pub struct Error(Box<error::Error>);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Pages<'a> {
    root: PagesTreeNode,
    stack: Vec<(std::vec::IntoIter<IndirectReference>, InheritableAttributes)>,
    current_iter: std::vec::IntoIter<IndirectReference>,
    current_inheritable: InheritableAttributes,
    objects: &'a mut Objects,
}

impl<'a> Pages<'a> {
    pub fn new(pages: &PagesTreeNode, objects: &'a mut Objects) -> Self {
        Self {
            root: pages.clone(),
            stack: Vec::new(),
            current_iter: pages.kids.clone().into_iter(),
            current_inheritable: pages.inheritable_attributes.clone(),
            objects,
        }
    }

    fn compute_next(&mut self) -> Result<Option<Page>> {
        loop {
            if let Some(kid_ref) = self.current_iter.next() {
                let kid_obj = self
                    .objects
                    .get_object(&kid_ref)
                    .context(error::ObjectNotFound {
                        reference: kid_ref,
                        field: "Pages",
                    })?;
                let dictionary = kid_obj.as_dictionary().context(error::InvalidKidType {
                    field: "Kids",
                    indirect_reference: kid_ref,
                })?;
                let node_type = dictionary
                    .get("Type")
                    .and_then(|obj| obj.as_name().ok())
                    .context(error::FieldNotFound { field: "Type" })?;

                match node_type {
                    "Page" => {
                        return Ok(Some(Page::from_dictionary(
                            dictionary,
                            &self.current_inheritable,
                            self.objects,
                        )?));
                    }
                    "Pages" => {
                        let new_node = PagesTreeNode::from_dictionary(
                            dictionary,
                            Some(self.current_inheritable.clone()),
                        )?;
                        let old_iter =
                            std::mem::replace(&mut self.current_iter, new_node.kids.into_iter());
                        let old_inheritable = std::mem::replace(
                            &mut self.current_inheritable,
                            new_node.inheritable_attributes,
                        );

                        self.stack.push((old_iter, old_inheritable));
                    }
                    _ => {
                        return Err(error::Error::UnexpectedNodeType {
                            got: node_type.to_string(),
                        }
                        .into());
                    }
                }
            } else if let Some((parent_iter, parent_inheritable)) = self.stack.pop() {
                self.current_iter = parent_iter;
                self.current_inheritable = parent_inheritable;
            } else {
                return Ok(None);
            }
        }
    }
}

impl<'a> std::iter::Iterator for Pages<'a> {
    type Item = std::result::Result<Page, crate::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.compute_next().context(crate::error::Pages) {
            Ok(Some(val)) => Some(Ok(val)),
            Ok(None) => None,
            Err(err) => Some(Err(err.into())),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (1, Some(self.root.leaf_count))
    }
}

#[derive(Debug)]
#[allow(dead_code)]
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

    last_modified: Option<Date>,
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
            .map(|contents| {
                let contents = contents.direct(objects);
                contents
                    .as_stream()
                    .map(|stream| vec![stream.clone()])
                    .or_else(|_| {
                        contents
                            .as_array()
                            .with_objects(objects)
                            .of(|obj| obj.as_stream().cloned())
                    })
                    .context(error::FailedResolveContents {
                        object: contents.into_owned(),
                    })
            })
            .transpose()?
            .unwrap_or_else(Vec::new);

        let resources = dictionary
            .get("Resources")
            .map(|object| object.direct(objects).as_dictionary().cloned())
            .transpose()
            .context(error::InvalidType { field: "Resources" })?
            .or_else(|| inheritable_attrs.resources.clone())
            .context(error::FieldNotFound { field: "Resources" })?;

        let media_box = dictionary
            .get("MediaBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArray { field: "MediaBox" })?
            .or_else(|| inheritable_attrs.media_box.clone())
            .context(error::FieldNotFound { field: "MediaBox" })?;

        let crop_box = dictionary
            .get("CropBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArray { field: "CropBox" })?
            .or_else(|| inheritable_attrs.crop_box.clone())
            .unwrap_or_else(|| media_box.clone());

        let bleed_box = dictionary
            .get("BleedBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArray { field: "BleedBox" })?
            .unwrap_or_else(|| crop_box.clone());

        let trim_box = dictionary
            .get("TrimBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArray { field: "TrimBox" })?
            .unwrap_or_else(|| crop_box.clone());

        let art_box = dictionary
            .get("ArtBox")
            .map(|object| object.as_array().rectangle())
            .transpose()
            .context(error::InvalidArray { field: "ArtBox" })?
            .unwrap_or_else(|| crop_box.clone());

        let rotate = dictionary
            .get("Rotate")
            .map(|object| object.as_integer())
            .transpose()
            .context(error::InvalidType { field: "Rotate" })?
            .or(inheritable_attrs.rotate)
            .unwrap_or(0);

        let user_unit = dictionary
            .get("UserUnit")
            .map(|object| object.as_float())
            .transpose()
            .context(error::InvalidType { field: "UserUnit" })?
            .unwrap_or(1.0);

        let last_modified = dictionary
            .get("LastModified")
            .map(|object| -> Result<Date> {
                Ok(object
                    .as_string()
                    .context(error::InvalidType {
                        field: "LastModified",
                    })?
                    .to_date()
                    .context(error::InvalidDate {
                        field: "LastModified",
                    })?)
            })
            .transpose()?;

        let box_color_info = dictionary
            .get("BoxColorInfo")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidType {
                field: "BoxColorInfo",
            })?;

        let group = dictionary
            .get("Group")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidType { field: "Group" })?;

        let thumb = dictionary
            .get("Thumb")
            .map(|object| object.as_stream().cloned())
            .transpose()
            .context(error::InvalidType { field: "Thumb" })?;

        let b = dictionary
            .get("B")
            .map(|object| object.as_array().generic())
            .transpose()
            .context(error::InvalidArray { field: "B" })?;

        let dur = dictionary
            .get("Dur")
            .map(|object| object.as_float())
            .transpose()
            .context(error::InvalidType { field: "Dur" })?;

        let trans = dictionary
            .get("Trans")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidType { field: "Trans" })?;

        let annots = dictionary
            .get("Annots")
            .map(|object| object.direct(objects).as_array().generic())
            .transpose()
            .context(error::InvalidArray { field: "Annots" })?;

        let aa = dictionary
            .get("AA")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidType { field: "AA" })?;

        let metadata = dictionary
            .get("Metadata")
            .map(|object| object.as_stream().cloned())
            .transpose()
            .context(error::InvalidType { field: "Metadata" })?;

        let piece_info = dictionary
            .get("PieceInfo")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidType { field: "PieceInfo" })?;

        let struct_parents = dictionary
            .get("StructParents")
            .map(|object| object.as_integer())
            .transpose()
            .context(error::InvalidType {
                field: "StructParents",
            })?;

        let id = dictionary
            .get("ID")
            .map(|object| object.as_string())
            .transpose()
            .context(error::InvalidType { field: "ID" })?
            .map(|s| s.as_bytes().to_vec());

        let pz = dictionary
            .get("PZ")
            .map(|object| object.as_float())
            .transpose()
            .context(error::InvalidType { field: "PZ" })?;

        let separation_info = dictionary
            .get("SeparationInfo")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidType {
                field: "SeparationInfo",
            })?;

        let tabs = dictionary
            .get("Tabs")
            .map(|object| object.as_name())
            .transpose()
            .context(error::InvalidType { field: "Tabs" })?
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
            .context(error::InvalidType {
                field: "TemplateInstantiated",
            })?
            .map(|s| s.to_string());

        let pres_steps = dictionary
            .get("PresSteps")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidType { field: "PresSteps" })?;

        let vp = dictionary
            .get("VP")
            .map(|object| object.as_dictionary().cloned())
            .transpose()
            .context(error::InvalidType { field: "VP" })?;

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
pub struct PagesTreeNode {
    _parent: Option<IndirectReference>,
    leaf_count: usize,
    kids: Vec<IndirectReference>,
    inheritable_attributes: InheritableAttributes,
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
            _parent: None,
            leaf_count: count,
            kids,
            inheritable_attributes,
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct InheritableAttributes {
    resources: Option<Dictionary>,
    media_box: Option<Rectangle>,
    crop_box: Option<Rectangle>,
    rotate: Option<u16>,
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

    use crate::types::{IndirectReference, Object};

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

        #[snafu(display(
            "Invalid object type for field `{field}`. Used ref: `{indirect_reference}`"
        ))]
        InvalidKidType {
            field: &'static str,
            indirect_reference: IndirectReference,
            source: crate::types::object::Error,
        },

        #[snafu(display("Invalid array data for field `{field}`"))]
        InvalidArray {
            field: &'static str,
            source: crate::types::array::Error,
        },

        #[snafu(display("Invalid date data for field `{field}`"))]
        InvalidDate {
            field: &'static str,
            source: crate::types::string::Error,
        },

        #[snafu(display("Object with reference `{reference}` for field `{field}` not found"))]
        ObjectNotFound {
            reference: IndirectReference,
            field: &'static str,
            source: crate::objects::Error,
        },

        #[snafu(display("Unexpected node type. Got = `{got}`. Expected `Page` or `Pages`]"))]
        UnexpectedNodeType { got: String },

        #[snafu(display("Failed to resolve contents: unexpected object `{object:?}`"))]
        FailedResolveContents {
            object: Object,
            source: crate::types::array::Error,
        },
    }
}
