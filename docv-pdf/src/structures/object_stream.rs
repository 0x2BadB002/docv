use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    parser::{object_stream_data_header, read_object},
    types::{IndirectReference, Object, Stream},
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct ObjectStream {
    ids: Vec<Entry>,
    first_offset: usize,
    _extends: Option<IndirectReference>,
    data: Vec<u8>,
}

#[derive(Debug)]
struct Entry {
    _id: usize,
    offset: usize,
}

impl ObjectStream {
    pub fn from_stream(mut stream: Stream) -> Result<Self> {
        stream
            .process_filters()
            .context(error::FiltersProcessingSnafu)?;

        let n = stream
            .dictionary
            .get("N")
            .with_context(|| error::FieldNotFoundSnafu {
                field: "N".to_string(),
            })?
            .as_integer()
            .with_context(|_| error::InvalidFieldSnafu {
                field: "N".to_string(),
            })?;

        let first = stream
            .dictionary
            .get("First")
            .with_context(|| error::FieldNotFoundSnafu {
                field: "First".to_string(),
            })?
            .as_integer()
            .with_context(|_| error::InvalidFieldSnafu {
                field: "First".to_string(),
            })?;

        let extends = stream
            .dictionary
            .get("Extends")
            .map(|object| object.as_indirect_ref())
            .transpose()
            .with_context(|_| error::InvalidFieldSnafu {
                field: "First".to_string(),
            })?
            .cloned();

        let ids = object_stream_data_header(&stream.data[..first], n)
            .ok()
            .context(error::ParseIdsSnafu)?
            .iter()
            .map(|(id, offset)| Entry {
                _id: *id,
                offset: *offset,
            })
            .collect::<Vec<_>>();

        Ok(Self {
            ids,
            first_offset: first,
            _extends: extends,
            data: stream.data,
        })
    }

    pub fn get_object_by_index(&self, index: usize) -> Result<Object> {
        let offset = self.ids[index].offset;

        let object = read_object(&self.data[(self.first_offset + offset)..])
            .ok()
            .context(error::ParseObjectSnafu)?;

        Ok(object)
    }
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Failed to process stream filters"))]
        FiltersProcessing { source: crate::types::StreamError },

        #[snafu(display("Field '{field}' not found"))]
        FieldNotFound { field: String },

        #[snafu(display("Field '{field}' has unexpected type"))]
        InvalidField {
            field: String,
            source: crate::types::ObjectError,
        },

        #[snafu(display("Failed to parse ids array"))]
        ParseIds,

        #[snafu(display("ID {id} not found in object stream"))]
        IdNotFound { id: usize },

        #[snafu(display("Failed to parse object"))]
        ParseObject,
    }
}
