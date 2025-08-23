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

    use std::io::Write;

    use flate2::{Compression, write::ZlibEncoder};

    use crate::types::{Dictionary, Numeric, Object, PdfString};

    #[test]
    fn test_process_filter() {
        struct TestCase {
            input: Object,
            expected_ok: bool,
            description: &'static str,
        }

        let cases = vec![
            TestCase {
                input: Object::Name("FlateDecode".to_string()),
                expected_ok: true,
                description: "Valid single filter",
            },
            TestCase {
                input: Object::Name("InvalidFilter".to_string()),
                expected_ok: false,
                description: "Invalid single filter",
            },
            TestCase {
                input: Object::Array(vec![
                    Object::Name("FlateDecode".to_string()),
                    Object::Name("FlateDecode".to_string()),
                ]),
                expected_ok: true,
                description: "Valid filter pipeline",
            },
            TestCase {
                input: Object::Array(vec![
                    Object::Name("InvalidFilter".to_string()),
                    Object::Name("FlateDecode".to_string()),
                ]),
                expected_ok: false,
                description: "Pipeline with invalid filter",
            },
            TestCase {
                input: Object::Numeric(Numeric::Integer(42)),
                expected_ok: false,
                description: "Invalid filter object type",
            },
        ];

        for case in cases {
            let result = process_filter(&case.input);
            assert_eq!(
                result.is_ok(),
                case.expected_ok,
                "Case '{}' failed: expected OK={}, got {:?}",
                case.description,
                case.expected_ok,
                result
            );
        }
    }

    #[test]
    fn test_apply_filter() {
        struct TestCase {
            data: Vec<u8>,
            filter: StreamFilterType,
            content_length: usize,
            expected_data: Option<Vec<u8>>,
            successful: bool,
            description: &'static str,
        }

        let cases = vec![
            TestCase {
                data: b"test".to_vec(),
                filter: StreamFilterType::None,
                content_length: 4,
                expected_data: Some(b"test".to_vec()),
                successful: true,
                description: "No filter applied",
            },
            TestCase {
                data: {
                    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(b"hello").unwrap();
                    encoder.finish().unwrap()
                },
                filter: StreamFilterType::FlateDecode,
                content_length: 5,
                expected_data: Some(b"hello".to_vec()),
                successful: true,
                description: "FlateDecode filter with valid data",
            },
            TestCase {
                data: vec![
                    120, 218, 45, 210, 181, 78, 4, 81, 20, 128, 225, 115, 89, 96, 97, 129, 93, 88,
                    24, 220, 221, 221, 221, 221, 221, 221, 165, 226, 5, 40, 169, 72, 40, 104, 160,
                    65, 42, 42, 18, 194, 66, 69, 120, 5, 10, 10, 66, 79, 195, 3, 208, 194, 220, 63,
                    183, 249, 114, 243, 159, 153, 201, 77, 206, 136, 136, 252, 253, 249, 137, 132,
                    200, 22, 110, 226, 4, 70, 227, 62, 238, 226, 17, 30, 40, 17, 143, 8, 103, 81,
                    209, 199, 159, 230, 60, 133, 74, 137, 99, 221, 20, 63, 156, 81, 226, 244, 153,
                    226, 192, 57, 244, 199, 0, 12, 68, 39, 6, 97, 48, 186, 48, 4, 67, 49, 12, 221,
                    74, 92, 223, 230, 155, 30, 156, 87, 226, 126, 183, 239, 35, 222, 27, 237, 197,
                    173, 153, 134, 227, 162, 146, 171, 83, 83, 34, 112, 73, 201, 93, 171, 41, 94,
                    92, 81, 114, 95, 161, 223, 125, 60, 179, 117, 60, 140, 154, 105, 36, 174, 42,
                    135, 239, 92, 247, 183, 31, 91, 231, 229, 181, 246, 246, 206, 214, 245, 57,
                    104, 158, 140, 194, 53, 229, 250, 242, 218, 221, 202, 248, 208, 102, 62, 153,
                    169, 133, 213, 152, 137, 89, 152, 141, 57, 152, 139, 121, 152, 143, 49, 88,
                    128, 93, 24, 139, 165, 88, 134, 113, 88, 142, 21, 24, 143, 53, 88, 133, 181,
                    152, 128, 137, 88, 135, 245, 216, 128, 141, 216, 132, 205, 216, 130, 237, 152,
                    132, 29, 216, 137, 201, 88, 140, 37, 152, 130, 149, 216, 138, 169, 216, 134,
                    133, 88, 132, 105, 152, 142, 25, 216, 141, 227, 216, 135, 61, 216, 139, 67, 56,
                    136, 253, 56, 128, 163, 56, 140, 35, 56, 134, 27, 56, 141, 147, 184, 128, 179,
                    184, 140, 235, 202, 202, 22, 189, 163, 92, 101, 118, 180, 135, 219, 202, 202,
                    251, 213, 61, 191, 215, 244, 67, 220, 81, 214, 243, 171, 238, 47, 30, 253, 231,
                    159, 216, 219, 255, 7, 239, 213, 57, 247,
                ],
                filter: StreamFilterType::FlateDecode,
                content_length: 834,
                expected_data: Some(vec![
                    0, 0, 0, 0, 255, 255, 2, 0, 0, 11, 0, 101, 2, 0, 0, 11, 0, 100, 2, 0, 0, 11, 0,
                    83, 2, 0, 0, 11, 0, 22, 2, 0, 0, 11, 0, 106, 2, 0, 0, 11, 0, 104, 2, 0, 0, 11,
                    0, 109, 2, 0, 0, 11, 0, 107, 1, 0, 0, 15, 0, 0, 2, 0, 0, 11, 0, 0, 1, 22, 115,
                    216, 0, 0, 2, 0, 0, 11, 0, 85, 2, 0, 0, 11, 0, 1, 1, 0, 3, 98, 0, 0, 2, 0, 0,
                    11, 0, 2, 2, 0, 0, 11, 0, 87, 1, 0, 7, 181, 0, 0, 2, 0, 0, 11, 0, 3, 2, 0, 0,
                    11, 0, 89, 2, 0, 0, 11, 0, 4, 2, 0, 0, 11, 0, 5, 2, 0, 0, 11, 0, 6, 2, 0, 0,
                    11, 0, 7, 2, 0, 0, 11, 0, 8, 2, 0, 0, 11, 0, 9, 2, 0, 0, 11, 0, 10, 2, 0, 0,
                    11, 0, 11, 2, 0, 0, 11, 0, 12, 2, 0, 0, 11, 0, 13, 2, 0, 0, 11, 0, 14, 1, 0,
                    10, 229, 0, 0, 2, 0, 0, 11, 0, 15, 2, 0, 0, 11, 0, 90, 1, 0, 14, 208, 0, 0, 1,
                    0, 18, 161, 0, 0, 1, 0, 149, 162, 0, 0, 2, 0, 0, 11, 0, 16, 2, 0, 0, 11, 0, 92,
                    1, 0, 155, 136, 0, 0, 2, 0, 0, 11, 0, 17, 2, 0, 0, 11, 0, 93, 1, 0, 163, 64, 0,
                    0, 2, 0, 0, 11, 0, 18, 2, 0, 0, 11, 0, 95, 1, 0, 169, 51, 0, 0, 1, 0, 178, 142,
                    0, 0, 1, 3, 175, 80, 0, 0, 2, 0, 0, 11, 0, 19, 2, 0, 0, 11, 0, 96, 1, 3, 181,
                    145, 0, 0, 1, 3, 191, 234, 0, 0, 1, 7, 154, 160, 0, 0, 1, 7, 162, 163, 0, 0, 1,
                    10, 216, 76, 0, 0, 2, 0, 0, 11, 0, 20, 2, 0, 0, 11, 0, 97, 1, 10, 218, 18, 0,
                    0, 1, 21, 35, 212, 0, 0, 1, 21, 36, 180, 0, 0, 2, 0, 0, 11, 0, 21, 2, 0, 0, 11,
                    0, 54, 2, 0, 0, 11, 0, 36, 2, 0, 0, 11, 0, 37, 2, 0, 0, 11, 0, 38, 2, 0, 0, 11,
                    0, 39, 2, 0, 0, 11, 0, 40, 2, 0, 0, 11, 0, 41, 2, 0, 0, 11, 0, 42, 2, 0, 0, 11,
                    0, 23, 2, 0, 0, 11, 0, 43, 2, 0, 0, 11, 0, 69, 2, 0, 0, 11, 0, 24, 2, 0, 0, 11,
                    0, 48, 2, 0, 0, 11, 0, 49, 2, 0, 0, 11, 0, 25, 2, 0, 0, 11, 0, 50, 2, 0, 0, 11,
                    0, 51, 2, 0, 0, 11, 0, 26, 2, 0, 0, 11, 0, 55, 2, 0, 0, 11, 0, 53, 2, 0, 0, 11,
                    0, 56, 2, 0, 0, 11, 0, 27, 2, 0, 0, 11, 0, 28, 2, 0, 0, 11, 0, 57, 2, 0, 0, 11,
                    0, 58, 2, 0, 0, 11, 0, 59, 2, 0, 0, 11, 0, 60, 2, 0, 0, 11, 0, 61, 2, 0, 0, 11,
                    0, 62, 2, 0, 0, 11, 0, 63, 2, 0, 0, 11, 0, 66, 2, 0, 0, 11, 0, 29, 2, 0, 0, 11,
                    0, 67, 2, 0, 0, 11, 0, 68, 2, 0, 0, 11, 0, 30, 2, 0, 0, 11, 0, 46, 2, 0, 0, 11,
                    0, 47, 2, 0, 0, 11, 0, 31, 2, 0, 0, 11, 0, 52, 2, 0, 0, 11, 0, 64, 2, 0, 0, 11,
                    0, 32, 2, 0, 0, 11, 0, 65, 2, 0, 0, 11, 0, 44, 2, 0, 0, 11, 0, 45, 2, 0, 0, 11,
                    0, 33, 2, 0, 0, 11, 0, 34, 2, 0, 0, 11, 0, 35, 2, 0, 0, 11, 0, 70, 2, 0, 0, 11,
                    0, 82, 2, 0, 0, 11, 0, 73, 2, 0, 0, 11, 0, 71, 2, 0, 0, 11, 0, 72, 2, 0, 0, 11,
                    0, 77, 2, 0, 0, 11, 0, 76, 2, 0, 0, 11, 0, 74, 2, 0, 0, 11, 0, 75, 2, 0, 0, 11,
                    0, 80, 2, 0, 0, 11, 0, 78, 2, 0, 0, 11, 0, 79, 2, 0, 0, 11, 0, 81, 2, 0, 0, 11,
                    0, 99, 2, 0, 0, 11, 0, 86, 2, 0, 0, 11, 0, 84, 2, 0, 0, 11, 0, 91, 2, 0, 0, 11,
                    0, 88, 2, 0, 0, 11, 0, 94, 2, 0, 0, 11, 0, 98, 1, 21, 38, 0, 0, 0, 1, 21, 40,
                    1, 0, 0, 2, 0, 0, 11, 0, 105, 2, 0, 0, 11, 0, 102, 1, 21, 41, 246, 0, 0, 1, 21,
                    42, 72, 0, 0, 2, 0, 0, 11, 0, 108, 2, 0, 0, 11, 0, 103, 1, 21, 182, 189, 0, 0,
                    1, 21, 183, 15, 0, 0, 1, 22, 128, 163, 0, 0,
                ]),
                successful: true,
                description: "FlateDecode with xref object stream",
            },
            TestCase {
                data: b"invalid compressed data".to_vec(),
                filter: StreamFilterType::FlateDecode,
                content_length: 100,
                expected_data: None,
                successful: false,
                description: "FlateDecode filter with invalid data",
            },
            TestCase {
                data: {
                    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(b"hello").unwrap();
                    let compressed = encoder.finish().unwrap();

                    let mut encoder2 = ZlibEncoder::new(Vec::new(), Compression::default());
                    encoder2.write_all(&compressed).unwrap();
                    encoder2.finish().unwrap()
                },
                filter: StreamFilterType::PipeLine(vec![
                    StreamFilterType::FlateDecode,
                    StreamFilterType::FlateDecode,
                ]),
                content_length: 5,
                expected_data: Some(b"hello".to_vec()),
                successful: true,
                description: "Filter pipeline with multiple FlateDecode",
            },
            TestCase {
                data: b"test".to_vec(),
                filter: StreamFilterType::PipeLine(vec![
                    StreamFilterType::None,
                    StreamFilterType::None,
                ]),
                content_length: 4,
                expected_data: Some(b"test".to_vec()),
                successful: true,
                description: "Filter pipeline with multiple None filters",
            },
        ];

        for case in cases {
            let result = apply_filter(&case.data, &case.filter, case.content_length);

            if case.successful {
                assert!(result.is_ok(), "Case '{}' should succeed", case.description);
                let result = result.unwrap();
                assert_eq!(
                    result,
                    case.expected_data.unwrap(),
                    "Case '{}' data mismatch. Expected len = {}. Got = {}",
                    case.description,
                    case.content_length,
                    result.len()
                );
            } else {
                assert!(result.is_err(), "Case '{}' should fail", case.description);
            }
        }
    }

    #[test]
    fn test_stream_process_filters() {
        struct TestCase {
            dictionary: Dictionary,
            data: Vec<u8>,
            expected_data: Option<Vec<u8>>,
            successful: bool,
            description: &'static str,
        }

        let cases = vec![
            TestCase {
                dictionary: Dictionary::from([
                    ("Type".to_string(), Object::Name("XObject".to_string())),
                    ("Length".to_string(), Object::Numeric(Numeric::Integer(4))),
                ]),
                data: b"test".to_vec(),
                expected_data: Some(b"test".to_vec()),
                successful: true,
                description: "Valid stream with no filter",
            },
            TestCase {
                dictionary: Dictionary::from([
                    ("Type".to_string(), Object::Name("XObject".to_string())),
                    (
                        "Field".to_string(),
                        Object::String(PdfString::Literal(String::from("123456"))),
                    ),
                    ("Length".to_string(), Object::Numeric(Numeric::Integer(4))),
                ]),
                data: b"test".to_vec(),
                expected_data: Some(b"test".to_vec()),
                successful: true,
                description: "Valid stream with no filter and custom field",
            },
            TestCase {
                dictionary: Dictionary::from([
                    ("Type".to_string(), Object::Name("XObject".to_string())),
                    (
                        "Filter".to_string(),
                        Object::Name("FlateDecode".to_string()),
                    ),
                    ("Length".to_string(), Object::Numeric(Numeric::Integer(5))),
                ]),
                data: {
                    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(b"hello").unwrap();
                    encoder.finish().unwrap()
                },
                expected_data: Some(b"hello".to_vec()),
                successful: true,
                description: "Valid stream with FlateDecode filter",
            },
            TestCase {
                dictionary: Dictionary::from([
                    ("Type".to_string(), Object::Name("XObject".to_string())),
                    (
                        "Filter".to_string(),
                        Object::Name("FlateDecode".to_string()),
                    ),
                ]),
                data: b"compressed".to_vec(),
                expected_data: None,
                successful: false,
                description: "Missing Length for FlateDecode",
            },
            TestCase {
                dictionary: Dictionary::from([
                    ("Type".to_string(), Object::Name("XObject".to_string())),
                    ("Length".to_string(), Object::Numeric(Numeric::Integer(4))),
                    (
                        "ColorSpace".to_string(),
                        Object::Name("DeviceRGB".to_string()),
                    ),
                ]),
                data: b"test".to_vec(),
                expected_data: Some(b"test".to_vec()),
                successful: true,
                description: "Handles additional dictionary entries",
            },
        ];

        for case in cases {
            let mut stream = Stream {
                dictionary: case.dictionary,
                data: case.data,
            };

            let result = stream.process_filters();

            if case.successful {
                assert!(result.is_ok(), "Case '{}' should succeed", case.description);
                assert_eq!(
                    stream.data,
                    case.expected_data.unwrap(),
                    "Case '{}' data mismatch",
                    case.description
                );
            } else {
                assert!(result.is_err(), "Case '{}' should fail", case.description);
            }
        }
    }
}
