use std::collections::BTreeMap;

use smol_str::SmolStr;
use snafu::{ResultExt, Snafu};

use crate::{
    objects::Objects,
    types::{Dictionary, Object},
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
#[allow(dead_code)]
enum Resource {
    ExtGState { object: Object },
    ColorSpace { object: Object },
    Pattern { object: Object },
    Shading { object: Object },
    XObject { object: Object },
    Font { object: Object },
    ProcSet,
    Properties { object: Object },
}

#[derive(Debug)]
pub struct Resources {
    data: BTreeMap<SmolStr, Resource>,
}

impl Resources {
    pub fn from_dictionary(dictionary: &Dictionary, objects: &mut Objects) -> Result<Self> {
        let ext_gstate = dictionary
            .get("ExtGState")
            .map(|object| object.direct(objects));
        let ext_gstate = ext_gstate
            .as_ref()
            .map(|object| object.as_dictionary())
            .transpose()
            .context(error::InvalidType { field: "ExtGState" })?;
        let ext_gstate = ext_gstate
            .iter()
            .flat_map(|dictionary| dictionary.iter())
            .map(|(name, object)| {
                (
                    name.clone(),
                    Resource::ExtGState {
                        object: object.clone(),
                    },
                )
            });

        let color_space = dictionary
            .get("ColorSpace")
            .map(|object| object.direct(objects));
        let color_space = color_space
            .as_ref()
            .map(|object| object.as_dictionary())
            .transpose()
            .context(error::InvalidType {
                field: "ColorSpace",
            })?;
        let color_space = color_space
            .iter()
            .flat_map(|dictionary| dictionary.iter())
            .map(|(name, object)| {
                (
                    name.clone(),
                    Resource::ColorSpace {
                        object: object.clone(),
                    },
                )
            });

        let pattern = dictionary
            .get("Pattern")
            .map(|object| object.direct(objects));
        let pattern = pattern
            .as_ref()
            .map(|object| object.as_dictionary())
            .transpose()
            .context(error::InvalidType { field: "Pattern" })?;
        let pattern =
            pattern
                .iter()
                .flat_map(|dictionary| dictionary.iter())
                .map(|(name, object)| {
                    (
                        name.clone(),
                        Resource::Pattern {
                            object: object.clone(),
                        },
                    )
                });

        let shading = dictionary
            .get("Shading")
            .map(|object| object.direct(objects));
        let shading = shading
            .as_ref()
            .map(|object| object.as_dictionary())
            .transpose()
            .context(error::InvalidType { field: "Shading" })?;
        let shading =
            shading
                .iter()
                .flat_map(|dictionary| dictionary.iter())
                .map(|(name, object)| {
                    (
                        name.clone(),
                        Resource::Shading {
                            object: object.clone(),
                        },
                    )
                });

        let x_object = dictionary
            .get("XObject")
            .map(|object| object.direct(objects));
        let x_object = x_object
            .as_ref()
            .map(|object| object.as_dictionary())
            .transpose()
            .context(error::InvalidType { field: "XObject" })?;
        let x_object = x_object
            .iter()
            .flat_map(|dictionary| dictionary.iter())
            .map(|(name, object)| {
                (
                    name.clone(),
                    Resource::XObject {
                        object: object.clone(),
                    },
                )
            });

        let font = dictionary.get("Font");
        let font = font
            .as_ref()
            .map(|object| object.as_dictionary())
            .transpose()
            .context(error::InvalidType { field: "Font" })?;
        let font = font
            .iter()
            .flat_map(|dictionary| dictionary.iter())
            .map(|(name, object)| {
                (
                    name.clone(),
                    Resource::Font {
                        object: object.clone(),
                    },
                )
            });

        let proc_set = dictionary
            .get("ProcSet")
            .map(|object| {
                object
                    .direct(objects)
                    .as_array()
                    .of(|object| object.as_name().cloned())
            })
            .transpose()
            .context(error::InvalidArray { field: "ProcSet" })?;
        let proc_set = proc_set
            .iter()
            .flat_map(|dictionary| dictionary.iter())
            .map(|name| ((**name).clone(), Resource::ProcSet));

        let properties = dictionary
            .get("Properties")
            .map(|object| object.direct(objects));
        let properties = properties
            .as_ref()
            .map(|object| object.as_dictionary())
            .transpose()
            .context(error::InvalidType {
                field: "Properties",
            })?;
        let properties = properties
            .iter()
            .flat_map(|dictionary| dictionary.iter())
            .map(|(name, object)| {
                (
                    name.clone(),
                    Resource::Properties {
                        object: object.clone(),
                    },
                )
            });

        Ok(Self {
            data: BTreeMap::from_iter(
                ext_gstate
                    .chain(color_space)
                    .chain(pattern)
                    .chain(shading)
                    .chain(x_object)
                    .chain(font)
                    .chain(proc_set)
                    .chain(properties),
            ),
        })
    }
}

impl std::fmt::Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Resource::ExtGState { .. } => write!(f, "ExtGState"),
            Resource::ColorSpace { .. } => write!(f, "ColorSpace"),
            Resource::Pattern { .. } => write!(f, "Pattern"),
            Resource::Shading { .. } => write!(f, "Shading"),
            Resource::XObject { .. } => write!(f, "XObject"),
            Resource::Font { .. } => write!(f, "Font"),
            Resource::ProcSet => write!(f, "ProcSet"),
            Resource::Properties { .. } => write!(f, "Properties"),
        }
    }
}

impl std::fmt::Display for Resources {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (name, resource) in self.data.iter() {
            writeln!(f, "{}: {}", name, resource)?;
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

        #[snafu(display("Invalid array object in field `{field}`"))]
        InvalidArray {
            field: &'static str,
            source: crate::types::array::Error,
        },
    }
}
