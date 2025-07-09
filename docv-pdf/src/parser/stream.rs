use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_until},
    multi::many0,
    sequence::{delimited, terminated},
};

use crate::parser::{
    DictionaryRecord, dictionary,
    whitespace::{comment, eol, whitespace},
};

/// Parses a PDF stream object from the input.
///
/// PDF streams consist of:
/// 1. A dictionary describing the stream properties
/// 2. The `stream` keyword followed by content data
/// 3. The `endstream` keyword
///
/// Allows optional whitespace, comments, and end-of-line markers
/// between the dictionary and `stream` keyword.
///
/// # Example
/// ```
/// <<
///     /Length 25
///     /Filter /ASCIIHexDecode
/// >>
/// stream
/// 68656c6c6f20776f726c64> % "hello world" in hex
/// endstream
/// ```
///
/// # Arguments
/// * `input` - Byte slice to parse
///
/// # Returns
/// `IResult` containing:
/// - Remaining input after parsing
/// - Tuple with two elements on success:
///   1. Iterator over stream dictionary records
///   2. Raw stream content bytes (data between `stream` and `endstream`)
pub fn stream(input: &[u8]) -> IResult<&[u8], (impl Iterator<Item = DictionaryRecord>, &[u8])> {
    let raw_data = take_until("endstream");

    let content = delimited(
        (tag("stream"), alt((tag("\r\n"), tag("\n")))),
        raw_data,
        tag("endstream"),
    );

    (
        terminated(dictionary, many0(alt((whitespace, comment, eol)))),
        content,
    )
        .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::dictionary::DictionaryRecord;
    use crate::parser::{Numeric, Object};
    use nom::error::dbg_dmp;

    #[test]
    fn test_stream_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_dict: Option<Vec<DictionaryRecord>>,
            expected_content: Option<&'static [u8]>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            // Valid streams
            TestCase {
                name: "valid minimal stream",
                input: b"<<>>stream\nendstream",
                expected: true,
                expected_dict: Some(vec![]),
                expected_content: Some(b""),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid stream with content",
                input: b"<< /Length 5 >>stream\nhelloendstream",
                expected: true,
                expected_dict: Some(vec![DictionaryRecord {
                    key: "Length".to_string(),
                    value: Object::Numeric(Numeric::Integer(5)),
                }]),
                expected_content: Some(b"hello"),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid stream with CRLF",
                input: b"<< /Length 5 >>stream\r\nhelloendstream",
                expected: true,
                expected_dict: Some(vec![DictionaryRecord {
                    key: "Length".to_string(),
                    value: Object::Numeric(Numeric::Integer(5)),
                }]),
                expected_content: Some(b"hello"),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid stream with comments and whitespace",
                input: b"<< /Length 5 >> % comment\n\t stream\nhelloendstream",
                expected: true,
                expected_dict: Some(vec![DictionaryRecord {
                    key: "Length".to_string(),
                    value: Object::Numeric(Numeric::Integer(5)),
                }]),
                expected_content: Some(b"hello"),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid stream with remainder",
                input: b"<< /Length 5 >>stream\nhelloendstreamrest",
                expected: true,
                expected_dict: Some(vec![DictionaryRecord {
                    key: "Length".to_string(),
                    value: Object::Numeric(Numeric::Integer(5)),
                }]),
                expected_content: Some(b"hello"),
                expected_remainder: Some(b"rest"),
            },
            // Invalid streams
            TestCase {
                name: "invalid missing stream keyword",
                input: b"<< /Length 5 >>helloendstream",
                expected: false,
                expected_dict: None,
                expected_content: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid missing endstream",
                input: b"<< /Length 5 >>stream\nhello",
                expected: false,
                expected_dict: None,
                expected_content: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid unclosed dictionary",
                input: b"<< /Length 5 stream\nhelloendstream",
                expected: false,
                expected_dict: None,
                expected_content: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid content before dictionary",
                input: b"prefix<<>>stream\nendstream",
                expected: false,
                expected_dict: None,
                expected_content: None,
                expected_remainder: None,
            },
        ];

        for case in &test_cases {
            let result = dbg_dmp(stream, "pdf_stream").parse(case.input);
            assert_eq!(
                result.is_ok(),
                case.expected,
                "Test '{}' failed: expected success: {}",
                case.name,
                case.expected,
            );

            if case.expected {
                let (actual_remainder, (dict_iter, content)) = result.unwrap();
                let dict_vec: Vec<DictionaryRecord> = dict_iter.collect();
                assert_eq!(
                    dict_vec,
                    *case.expected_dict.as_ref().unwrap(),
                    "Test '{}' failed: expected dictionary: {:?}, got: {:?}",
                    case.name,
                    case.expected_dict,
                    dict_vec
                );
                assert_eq!(
                    content,
                    *case.expected_content.as_ref().unwrap(),
                    "Test '{}' failed: expected content: {:?}, got: {:?}",
                    case.name,
                    case.expected_content,
                    content
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
