use nom::{
    Finish, IResult, ParseTo, Parser,
    branch::alt,
    bytes::complete::{tag, take, take_until},
    character::complete::digit1,
    combinator::{opt, value},
    error::Error,
    multi::many0,
    sequence::{delimited, preceded, separated_pair, terminated},
};

use crate::{
    parser::{
        dictionary::dictionary,
        indirect_object::indirect_object,
        stream::stream,
        whitespace::{comment, eol, whitespace},
    },
    types::{Dictionary, IndirectObject, Stream},
};

/// Represents a cross-reference table or object stream in a PDF document.
///
/// PDF documents can store cross-reference information in two formats:
/// 1. Traditional cross-reference table with explicit offset entries
/// 2. Object stream containing compressed cross-reference data
#[derive(Debug, Clone)]
pub enum XrefObject {
    /// Traditional cross-reference table format
    Table(Vec<XrefTableSection>),
    /// Compressed cross-reference stream object
    ObjectStream(Stream),
    /// Indirect object definition with compressed cross-reference stream object
    IndirectObjectStream(IndirectObject),
}

/// Represents a section of a cross-reference table.
///
/// Cross-reference tables are divided into sections, each containing
/// a contiguous range of object entries with the same generation number range.
#[derive(Debug, Clone)]
pub struct XrefTableSection {
    pub first_id: usize,
    pub _length: usize,
    pub entries: Vec<XrefTableEntry>,
}

/// Represents a single entry in a cross-reference table section.
///
/// Each entry describes the location and status of a PDF object.
#[derive(Debug, Clone)]
pub struct XrefTableEntry {
    pub offset: usize,
    pub gen_id: usize,
    pub occupied: bool,
}

/// Parses the `startxref` keyword and returns the byte offset of the last cross-reference section.
///
/// The `startxref` keyword should be followed by:
/// 1. An integer offset
/// 2. The `%%EOF` trailer marker
///
/// Allows optional content before the keyword and requires proper line endings.
///
/// # Example
/// ```
/// startxref
/// 12345
/// %%EOF
/// ```
///
/// # Arguments
/// * `input` - Byte slice to parse
///
/// # Returns
/// `IResult` containing:
/// - Remaining input after parsing
/// - Offset value on success
pub fn read_startxref(input: &[u8]) -> Result<(&[u8], u64), Error<&[u8]>> {
    let value = digit1.map_opt(|res: &[u8]| res.parse_to());

    preceded(
        take_until("startxref"),
        delimited((tag("startxref"), eol), value, (eol, tag("%%EOF"))),
    )
    .parse(input)
    .finish()
}

/// Parses either a cross-reference table or an object stream containing cross-references.
///
/// PDF cross-references can be stored in two formats:
/// 1. Traditional cross-reference table
/// 2. Object stream (compressed cross-reference stream)
///
/// # Example
/// ```
/// xref
/// 0 1
/// 0000000000 65535 f
/// ```
///
/// # Arguments
/// * `input` - Byte slice to parse
///
/// # Returns
/// `IResult` containing:
/// - Remaining input after parsing
/// - `Xref` enum variant representing either a table or object stream
pub fn read_xref(input: &[u8]) -> Result<(&[u8], XrefObject), Error<&[u8]>> {
    alt((
        xref_table.map(XrefObject::Table),
        stream.map(XrefObject::ObjectStream),
        indirect_object.map(XrefObject::IndirectObjectStream),
    ))
    .parse(input)
    .finish()
}

/// Parses the trailer dictionary containing document-wide information.
///
/// The trailer dictionary typically contains:
/// - Document catalog reference
/// - Cross-reference table information
/// - Encryption metadata
///
/// Should appear after the last cross-reference section.
///
/// # Example
/// ```
/// trailer
/// << /Size 22 /Root 1 0 R >>
/// ```
///
/// # Arguments
/// * `input` - Byte slice to parse
///
/// # Returns
/// `IResult` containing:
/// - Remaining input after parsing
/// - Dictionary containing trailer information
pub fn read_trailer(input: &[u8]) -> Result<(&[u8], Dictionary), Error<&[u8]>> {
    let trailer = delimited(
        many0(alt((whitespace, eol, comment))),
        tag("trailer"),
        many0(alt((whitespace, eol, comment))),
    );

    preceded(trailer, dictionary).parse(input).finish()
}

/// Parses a cross-reference table from the input.
///
/// Cross-reference tables consist of one or more sections, each containing:
/// 1. A header with starting object ID and entry count
/// 2. A series of entries with format: `offset gen_id status`
///
/// # Format
/// Each entry is 20 bytes long:
/// - Bytes 0-9: Offset (10-digit number)
/// - Byte 10: Space
/// - Bytes 11-15: Generation number (5-digit number)
/// - Byte 16: Space
/// - Byte 17: Status ('n' for in-use, 'f' for free)
/// - Byte 18: Optional space
/// - Bytes 19-20: Line ending
///
/// # Arguments
/// * `input` - Byte slice to parse
///
/// # Returns
/// `IResult` containing:
/// - Remaining input after parsing
/// - Iterator over cross-reference table sections
fn xref_table(input: &[u8]) -> IResult<&[u8], Vec<XrefTableSection>> {
    let entry = (
        take(10usize).map_opt(|res: &[u8]| res.parse_to()),
        value((), tag(" ")),
        take(5usize).map_opt(|res: &[u8]| res.parse_to()),
        value((), tag(" ")),
        alt((value(true, tag("n")), value(false, tag("f")))),
        opt(tag(" ")),
        eol,
    )
        .map(|(offset, _, gen_id, _, occupied, _, _)| XrefTableEntry {
            offset,
            gen_id,
            occupied,
        });

    let subsection = (
        terminated(
            separated_pair(
                digit1.map_opt(|res: &[u8]| res.parse_to()),
                tag(" "),
                digit1.map_opt(|res: &[u8]| res.parse_to()),
            ),
            eol,
        ),
        many0(entry),
    )
        .map(|((first_id, length), entries)| XrefTableSection {
            first_id,
            _length: length,
            entries,
        });

    preceded((tag("xref"), eol), many0(subsection)).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Dictionary, IndirectReference, Numeric, Object};
    use std::collections::BTreeMap;

    #[test]
    fn test_startxref_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_value: Option<u64>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            // Valid startxref
            TestCase {
                name: "valid startxref",
                input: b"startxref\n12345\n%%EOF",
                expected: true,
                expected_value: Some(12345),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid startxref with content before",
                input: b"some content\nstartxref\n67890\n%%EOF",
                expected: true,
                expected_value: Some(67890),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid startxref with remainder",
                input: b"startxref\n9999\n%%EOFremaining",
                expected: true,
                expected_value: Some(9999),
                expected_remainder: Some(b"remaining"),
            },
            // Invalid startxref
            TestCase {
                name: "invalid missing startxref",
                input: b"12345\n%%EOF",
                expected: false,
                expected_value: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid missing %%EOF",
                input: b"startxref\n12345",
                expected: false,
                expected_value: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid non-numeric offset",
                input: b"startxref\nabc\n%%EOF",
                expected: false,
                expected_value: None,
                expected_remainder: None,
            },
        ];

        for case in &test_cases {
            let result = read_startxref(case.input);
            assert_eq!(
                result.is_ok(),
                case.expected,
                "Test '{}' failed: expected success: {}",
                case.name,
                case.expected,
            );

            if case.expected {
                let (actual_remainder, actual_value) = result.unwrap();
                assert_eq!(
                    actual_value,
                    case.expected_value.unwrap(),
                    "Test '{}' failed: expected value: {:?}, got: {:?}",
                    case.name,
                    case.expected_value,
                    actual_value
                );
                assert_eq!(
                    actual_remainder,
                    case.expected_remainder.unwrap(),
                    "Test '{}' failed: expected remainder: {:?}, got: {:?}",
                    case.name,
                    case.expected_remainder,
                    actual_remainder
                );
            }
        }
    }

    #[test]
    fn test_xref_table_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_sections: Option<usize>,
            expected_entries: Option<usize>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            // Valid xref tables
            TestCase {
                name: "valid minimal xref table",
                input: b"xref\n0 1\n0000000000 65535 f \n",
                expected: true,
                expected_sections: Some(1),
                expected_entries: Some(1),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid xref table with multiple sections",
                input: b"xref\n0 2\n0000000000 65535 f \n0000000010 00001 n \n3 1\n0000000020 00002 n \n",
                expected: true,
                expected_sections: Some(2),
                expected_entries: Some(3),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid xref table with remainder",
                input: b"xref\n0 1\n0000000000 65535 f \ntrailer",
                expected: true,
                expected_sections: Some(1),
                expected_entries: Some(1),
                expected_remainder: Some(b"trailer"),
            },
            // Invalid xref tables
            TestCase {
                name: "invalid missing xref keyword",
                input: b"0 1\n0000000000 65535 f \n",
                expected: false,
                expected_sections: None,
                expected_entries: None,
                expected_remainder: None,
            },
        ];

        for case in &test_cases {
            let result = read_xref(case.input);
            assert_eq!(
                result.is_ok(),
                case.expected,
                "Test '{}' failed: expected success: {}",
                case.name,
                case.expected,
            );

            if case.expected {
                let (actual_remainder, actual_xref) = result.unwrap();

                if let XrefObject::Table(sections) = actual_xref {
                    let total_entries: Vec<XrefTableEntry> = sections
                        .clone()
                        .into_iter()
                        .flat_map(|section| section.entries)
                        .collect();

                    assert_eq!(
                        sections.len(),
                        case.expected_sections.unwrap(),
                        "Test '{}' failed: expected sections: {:?}, got: {:?}",
                        case.name,
                        case.expected_sections,
                        sections.len()
                    );
                    assert_eq!(
                        total_entries.len(),
                        case.expected_entries.unwrap(),
                        "Test '{}' failed: expected entries: {:?}, got: {:?}, total_entries: {:?}",
                        case.name,
                        case.expected_entries.unwrap(),
                        total_entries.len(),
                        total_entries,
                    );
                    assert_eq!(
                        actual_remainder,
                        case.expected_remainder.unwrap(),
                        "Test '{}' failed: expected remainder: {:?}, got: {:?}",
                        case.name,
                        case.expected_remainder,
                        actual_remainder
                    );
                } else {
                    panic!(
                        "Test '{}' failed: expected Xref::Table, got different variant. Got: {:?}",
                        case.name, actual_xref,
                    );
                }
            }
        }
    }

    #[test]
    fn test_trailer_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_dict: Option<Dictionary>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            // Valid trailers
            TestCase {
                name: "valid minimal trailer",
                input: b"trailer<<>>",
                expected: true,
                expected_dict: Some(Dictionary {
                    records: BTreeMap::new(),
                }),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid trailer with dictionary",
                input: b"trailer<</Size 10/Root 1 0 R>>",
                expected: true,
                expected_dict: Some(Dictionary {
                    records: BTreeMap::from([
                        ("Size".to_string(), Object::Numeric(Numeric::Integer(10))),
                        (
                            "Root".to_string(),
                            Object::IndirectReference(IndirectReference { id: 1, gen_id: 0 }),
                        ),
                    ]),
                }),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid trailer with whitespace",
                input: b"trailer \n\t << /Size 10 >>",
                expected: true,
                expected_dict: Some(Dictionary {
                    records: BTreeMap::from([(
                        "Size".to_string(),
                        Object::Numeric(Numeric::Integer(10)),
                    )]),
                }),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid trailer with whitespace in begining",
                input: b"\n\t trailer \n\t << /Size 10 >>",
                expected: true,
                expected_dict: Some(Dictionary {
                    records: BTreeMap::from([(
                        "Size".to_string(),
                        Object::Numeric(Numeric::Integer(10)),
                    )]),
                }),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid trailer with remainder",
                input: b"trailer<</Size 10>>startxref",
                expected: true,
                expected_dict: Some(Dictionary {
                    records: BTreeMap::from([(
                        "Size".to_string(),
                        Object::Numeric(Numeric::Integer(10)),
                    )]),
                }),
                expected_remainder: Some(b"startxref"),
            },
            // Invalid trailers
            TestCase {
                name: "invalid missing trailer keyword",
                input: b"<< /Size 10 >>",
                expected: false,
                expected_dict: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid malformed dictionary",
                input: b"trailer<</Size 10",
                expected: false,
                expected_dict: None,
                expected_remainder: None,
            },
        ];

        for case in &test_cases {
            let result = read_trailer(case.input);
            assert_eq!(
                result.is_ok(),
                case.expected,
                "Test '{}' failed: expected success: {}",
                case.name,
                case.expected,
            );

            if case.expected {
                let (actual_remainder, actual_dict) = result.unwrap();
                assert_eq!(
                    actual_dict,
                    *case.expected_dict.as_ref().unwrap(),
                    "Test '{}' failed: expected dictionary: {:?}, got: {:?}",
                    case.name,
                    case.expected_dict,
                    actual_dict
                );
                assert_eq!(
                    actual_remainder,
                    case.expected_remainder.unwrap(),
                    "Test '{}' failed: expected remainder: {:?}, got: {:?}",
                    case.name,
                    case.expected_remainder,
                    actual_remainder
                );
            }
        }
    }
}
