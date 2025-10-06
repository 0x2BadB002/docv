use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    parser::{read_object, read_object_stream_header},
    types::{IndirectReference, Object, Stream},
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

/// A PDF object stream that contains multiple compressed objects in a single stream.
///
/// Object streams are used in PDF 1.5+ to compress multiple objects into a single stream,
/// reducing file size and improving performance. They contain an index of object IDs
/// and offsets followed by the compressed object data.
#[derive(Debug)]
pub struct ObjectStream {
    ids: Vec<Entry>,
    first_offset: usize,
    _extends: Option<IndirectReference>,
    data: Vec<u8>,
}

/// An entry in the object stream index mapping an object ID to its data offset.
#[derive(Debug)]
struct Entry {
    _id: usize,
    offset: usize,
}

impl ObjectStream {
    pub fn from_stream(mut stream: Stream) -> Result<Self> {
        stream.process_filters().context(error::FiltersProcessing)?;

        let n = stream
            .dictionary
            .get("N")
            .context(error::FieldNotFound { field: "N" })?
            .as_integer()
            .context(error::InvalidField { field: "N" })?;

        let first = stream
            .dictionary
            .get("First")
            .context(error::FieldNotFound { field: "First" })?
            .as_integer()
            .context(error::InvalidField { field: "First" })?;

        let extends = stream
            .dictionary
            .get("Extends")
            .map(|object| object.as_indirect_ref().cloned())
            .transpose()
            .context(error::InvalidField { field: "Extends" })?;

        let ids = read_object_stream_header(&stream.data[..first], n)
            .ok()
            .context(error::ParseIds)?
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

    /// Retrieves an object from the stream by its index position.
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the object in the stream's internal index
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The index is out of bounds
    /// - The object data at the calculated offset cannot be parsed
    pub fn get_object_by_index(&self, index: usize) -> Result<Object> {
        let offset = self.ids[index].offset;

        let object = read_object(&self.data[(self.first_offset + offset)..])
            .ok()
            .context(error::ParseObject)?;

        Ok(object)
    }
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Failed to process stream filters"))]
        FiltersProcessing { source: crate::types::stream::Error },

        #[snafu(display("Field '{field}' not found"))]
        FieldNotFound { field: &'static str },

        #[snafu(display("Field '{field}' has unexpected type"))]
        InvalidField {
            field: &'static str,
            source: crate::types::object::Error,
        },

        #[snafu(display("Failed to parse ids array"))]
        ParseIds,

        #[snafu(display("ID {id} not found in object stream"))]
        IdNotFound { id: usize },

        #[snafu(display("Failed to parse object"))]
        ParseObject,
    }
}
