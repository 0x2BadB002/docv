use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    objects::Objects,
    structures::root::pages_tree::InheritableAttributes,
    types::{Array, Dictionary, Rectangle, Stream, string::Date},
};

#[derive(Debug, Snafu)]
#[snafu(source(from(error::Error, Box::new)))]
pub struct Error(Box<error::Error>);
type Result<T> = std::result::Result<T, Error>;

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
    pub fn from_dictionary(
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

impl std::fmt::Display for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "--- Content ---")?;
        for content in self.contents.iter() {
            write!(f, "\n{}", content)?;
        }
        writeln!(f, "--- End Content ---\n")?;

        if let Some(metadata) = self.metadata.as_ref() {
            writeln!(f, "--- Metadata ---")?;
            write!(f, "{}", metadata)?;
            writeln!(f, "--- End Metadata ---\n")?;
        }

        writeln!(f, "user_unit: {}, ", self.user_unit)?;
        writeln!(f, "rotate: {}, ", self.rotate)?;
        writeln!(f, "media_box: {}, ", self.media_box)?;
        writeln!(f, "crop_box: {}, ", self.crop_box)?;
        writeln!(f, "bleed_box: {}, ", self.bleed_box)?;
        writeln!(f, "trim_box: {}, ", self.trim_box)?;
        writeln!(f, "art_box: {}", self.art_box)?;
        writeln!(f, "tabs: {}", self.tabs)?;
        if let Some(ref date) = self.last_modified {
            writeln!(f, "last_modified: {}", date)?;
        }
        if let Some(dur) = self.dur {
            writeln!(f, "dur: {}", dur)?;
        }
        if let Some(ref id) = self.id {
            writeln!(f, "id: {:?}", id)?;
        }
        if let Some(pz) = self.pz {
            writeln!(f, "pz: {}", pz)?;
        }
        if let Some(ref template) = self.template_instantiated {
            writeln!(f, "template_instantiated: {}", template)?;
        }
        if let Some(struct_parents) = self.struct_parents {
            writeln!(f, "struct_parents: {}", struct_parents)?;
        }

        Ok(())
    }
}

impl std::fmt::Display for TabOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TabOrder::Row => write!(f, "Row"),
            TabOrder::Column => write!(f, "Column"),
            TabOrder::Structure => write!(f, "Structure"),
            TabOrder::None => write!(f, "None"),
        }
    }
}

mod error {
    use snafu::Snafu;

    use crate::types::Object;

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

        #[snafu(display("Invalid date data for field `{field}`"))]
        InvalidDate {
            field: &'static str,
            source: crate::types::string::Error,
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

#[cfg(test)]
mod test {
    use snafu::Whatever;

    use crate::Document;

    use super::*;
    use std::{fs, path::PathBuf, sync::LazyLock};

    static EXAMPLES: LazyLock<PathBuf> = LazyLock::new(|| {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.pop();
        dir.push("example_files");
        dir
    });

    #[snafu::report]
    #[test]
    fn print_pages_example_files() -> std::result::Result<(), Whatever> {
        for example in
            fs::read_dir(EXAMPLES.clone()).whatever_context("Failed to read directory")?
        {
            let entry = example.whatever_context("Failed to directory entry")?;
            let path = entry.path();

            let mut document = Document::from_path(&path)
                .with_whatever_context(|_| format!("Failed to open file {}", path.display()))?;

            let result = document
                .pages()
                .collect::<std::result::Result<Vec<_>, _>>()
                .with_whatever_context(|_| {
                    format!("Failed to iterate over pages for file {}", path.display())
                })?;

            for page in result.iter() {
                let _ = format!("{}", page);
            }
        }
        Ok(())
    }
}
