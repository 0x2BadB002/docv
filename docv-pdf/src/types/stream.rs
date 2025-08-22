use std::io::Read;

use flate2::read::ZlibDecoder;
use snafu::{OptionExt, ResultExt, Snafu};

use crate::types::{Dictionary, Object};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

/// Represents a PDF stream object containing both a dictionary and binary data.
///
/// PDF streams are used to store large amounts of data, such as:
/// - Image and font data
/// - Content streams for page descriptions
/// - Compressed object data
/// - Metadata and embedded files
///
/// # Structure
/// A stream consists of:
/// 1. A dictionary specifying stream properties (length, filters, etc.)
/// 2. Binary data that may be compressed or encoded
///
/// # Filter Support
/// Currently supports:
/// - No filtering (raw data)
/// - FlateDecode (zlib/deflate compression)
/// - Filter pipelines (multiple filters applied in sequence)
///
/// # Example
/// ```
/// <<
///   /Length 128
///   /Filter /FlateDecode
/// >>
/// stream
/// ...compressed binary data...
/// endstream
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct Stream {
    pub dictionary: Dictionary,
    pub data: Vec<u8>,
}

/// Represents the type of filter applied to a stream's data.
///
/// PDF streams can have multiple filters applied in sequence (pipeline)
/// to compress or encode the data. Each filter must be applied in reverse
/// order during decoding.
#[derive(Debug, Default)]
enum StreamFilterType {
    /// No filtering applied - raw data
    #[default]
    None,
    /// FlateDecode compression (zlib/deflate algorithm)
    FlateDecode,
    /// Multiple filters applied in sequence
    PipeLine(Vec<StreamFilterType>),
}

impl Stream {
    /// Processes all filters applied to the stream data and decompresses/decodes it.
    ///
    /// This method reads the filter information from the stream's dictionary,
    /// applies the appropriate decoding algorithms, and replaces the stream's
    /// data with the processed result.
    ///
    /// # Steps
    /// 1. Extracts the content length from the dictionary
    /// 2. Parses the filter specification (single filter or pipeline)
    /// 3. Applies filters in reverse order (as per PDF specification)
    /// 4. Replaces the internal data with the processed result
    ///
    /// # Returns
    /// - `Ok(())` if processing completed successfully
    /// - `Err(Error)` if any step fails (missing length, unsupported filter, etc.)
    ///
    /// # Errors
    /// Returns an error if:
    /// - The Length entry is missing from the dictionary
    /// - The Length value cannot be converted to an integer
    /// - An unsupported filter is specified
    /// - Decompression fails (corrupted data, etc.)
    ///
    /// # Example
    /// ```
    /// let mut stream = Stream { ... };
    /// stream.process_filters()?; // Decompresses if FlateDecode filter is present
    /// ```
    pub fn process_filters(&mut self) -> Result<()> {
        let content_length = self
            .dictionary
            .get("Length")
            .with_context(|| error::NoStreamLengthSnafu)?
            .as_integer()
            .with_context(|_| error::UnexpectedDictionaryValueSnafu)?;

        let filter = match self.dictionary.get("Filter") {
            Some(object) => process_filter(object)?,
            None => StreamFilterType::default(),
        };

        self.data = apply_filter(&self.data, &filter, content_length)?;

        Ok(())
    }
}

/// Parses a filter specification from a PDF object into a StreamFilterType.
///
/// PDF filters can be specified as:
/// - A single name object (e.g., `/FlateDecode`)
/// - An array of names for filter pipelines (e.g., `[/ASCII85Decode /FlateDecode]`)
///
/// # Arguments
/// * `filter` - PDF object containing the filter specification
///
/// # Returns
/// - `Ok(StreamFilterType)` representing the parsed filter(s)
/// - `Err(Error)` if the object format is invalid or contains unsupported filters
///
/// # Errors
/// Returns an error if:
/// - The object is not a name or array
/// - An unsupported filter name is encountered
fn process_filter(filter: &Object) -> Result<StreamFilterType> {
    match filter {
        Object::Name(name) => match name.as_str() {
            "FlateDecode" => Ok(StreamFilterType::FlateDecode),
            _ => Err(error::Error::InvalidStreamFilter { name: name.clone() }.into()),
        },
        Object::Array(pipeline) => Ok(StreamFilterType::PipeLine(
            pipeline
                .iter()
                .map(process_filter)
                .collect::<Result<Vec<_>>>()?,
        )),
        _ => Err(error::Error::InvalidStreamFiltersObject {
            object: filter.clone(),
        }
        .into()),
    }
}

/// Applies a filter (or filter pipeline) to stream data.
///
/// This function handles the actual decoding/decompression of stream data
/// according to the specified filter type. For filter pipelines, applies
/// filters in reverse order (last filter applied first during decoding).
///
/// # Arguments
/// * `data` - The raw stream data to process
/// * `filter` - The filter type to apply
/// * `content_length` - Expected length of decompressed data (for allocation)
///
/// # Returns
/// - `Ok(Vec<u8>)` containing the processed data
/// - `Err(Error)` if decompression fails or an unsupported filter is encountered
///
/// # Errors
/// Returns an error if:
/// - FlateDecode decompression fails (corrupted data, etc.)
/// - An unsupported filter type is specified
fn apply_filter(data: &[u8], filter: &StreamFilterType, content_length: usize) -> Result<Vec<u8>> {
    match filter {
        StreamFilterType::None => Ok(data.to_vec()),
        StreamFilterType::FlateDecode => {
            let mut decoder = ZlibDecoder::new(data);
            let mut data = Vec::with_capacity(content_length);

            decoder
                .read_to_end(&mut data)
                .context(error::DecompressionSnafu)?;

            Ok(data)
        }
        StreamFilterType::PipeLine(filters) => {
            filters.iter().try_fold(data.to_vec(), |data, filter| {
                apply_filter(&data, filter, content_length)
            })
        }
    }
}

mod error {
    use snafu::Snafu;

    use crate::types::object::Object;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Unexpected dictionary value"))]
        UnexpectedDictionaryValue { source: crate::types::object::Error },

        #[snafu(display("Stream content length not present"))]
        NoStreamLength,

        #[snafu(display("Unsupported stream filter {name}"))]
        InvalidStreamFilter { name: String },

        #[snafu(display("Unsupported stream filters object. Object =  {object:?}"))]
        InvalidStreamFiltersObject { object: Object },

        #[snafu(display("Error during decompression"))]
        Decompression { source: std::io::Error },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use flate2::Compression;
    use flate2::write::ZlibEncoder;

    use crate::types::{Dictionary, Numeric, Object};
}
