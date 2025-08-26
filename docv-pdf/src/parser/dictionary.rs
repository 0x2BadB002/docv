use std::collections::BTreeMap;

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    multi::many0,
    sequence::{delimited, preceded},
};

use crate::{
    parser::{
        name::name,
        object::object,
        whitespace::{comment, eol, whitespace},
    },
    types::Dictionary,
};

/// Parses a PDF dictionary from the input.
///
/// PDF dictionaries are enclosed in double angle brackets (`<<` and `>>`) and contain
/// one or more key-value pairs. Allows optional whitespace, comments, and end-of-line
/// markers between elements.
///
/// # Example
/// <<
///     /Key1 42
///     /Key2 (Text value)
///     % This is a comment
///     /Key3 [1 2 3]  % Array value
/// >>
///
/// # Arguments
/// * `input` - Byte slice to parse
///
/// # Returns
/// `IResult` containing:
/// - Remaining input after parsing
/// - Dictionary key-value pairs on success
pub fn dictionary(input: &[u8]) -> IResult<&[u8], Dictionary> {
    let key_value = (
        name,
        preceded(many0(alt((whitespace, comment, eol))), object),
    );

    let contents = many0(delimited(
        many0(alt((whitespace, comment, eol))),
        key_value,
        many0(alt((whitespace, comment, eol))),
    ))
    .map(|res| Dictionary {
        records: BTreeMap::from_iter(res.iter().cloned()),
    });

    delimited(tag("<<"), contents, tag(">>")).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Numeric, Object, PdfString};

    #[test]
    fn test_dictionary_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_result: Option<Dictionary>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            // Valid dictionaries
            TestCase {
                name: "valid empty dictionary",
                input: b"<<>>",
                expected: true,
                expected_result: Some(Dictionary::from([])),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid dictionary with one key-value pair",
                input: b"<< /Key 42 >>",
                expected: true,
                expected_result: Some(Dictionary::from([(
                    "Key".to_string(),
                    Object::Numeric(Numeric::Integer(42)),
                )])),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid dictionary with multiple key-value pairs",
                input: b"<< /Key1 1 /Key2 (two) /Key3 /three >>",
                expected: true,
                expected_result: Some(Dictionary::from([
                    ("Key1".to_string(), Object::Numeric(Numeric::Integer(1))),
                    (
                        "Key2".to_string(),
                        Object::String(PdfString::Literal("two".to_string())),
                    ),
                    ("Key3".to_string(), Object::Name("three".to_string())),
                ])),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid dictionary with comments and whitespace",
                input: b"<< % comment\n /Key1 1 % another\n /Key2 (two) \t /Key3 /three \n>>",
                expected: true,
                expected_result: Some(Dictionary::from([
                    ("Key1".to_string(), Object::Numeric(Numeric::Integer(1))),
                    (
                        "Key2".to_string(),
                        Object::String(PdfString::Literal("two".to_string())),
                    ),
                    ("Key3".to_string(), Object::Name("three".to_string())),
                ])),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid dictionary with nested structures",
                input: b"<< /Array [1 2 3] /Nested << /SubKey true >> >>",
                expected: true,
                expected_result: Some(Dictionary::from([
                    (
                        "Array".to_string(),
                        Object::Array(
                            vec![
                                Object::Numeric(Numeric::Integer(1)),
                                Object::Numeric(Numeric::Integer(2)),
                                Object::Numeric(Numeric::Integer(3)),
                            ]
                            .into(),
                        ),
                    ),
                    (
                        "Nested".to_string(),
                        Object::Dictionary(Dictionary::from([(
                            "SubKey".to_string(),
                            Object::Boolean(true),
                        )])),
                    ),
                ])),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "dictionary with remainder",
                input: b"<< /Key 42 >>rest",
                expected: true,
                expected_result: Some(Dictionary::from([(
                    "Key".to_string(),
                    Object::Numeric(Numeric::Integer(42)),
                )])),
                expected_remainder: Some(b"rest"),
            },
            TestCase {
                name: "partial xref dictionary",
                input: b"<</Type/XRef/Size 139>>",
                expected: true,
                expected_result: Some(Dictionary::from([
                    ("Type".to_string(), Object::Name(String::from("XRef"))),
                    ("Size".to_string(), Object::Numeric(Numeric::Integer(139))),
                ])),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "partial info dictionary",
                input: b"<<
                    % XML Metadata
                    /CreationDate (D:20211230134641+11'00')
                    /Creator      (By hand)
                    /ModDate      (D:20211230134824+11'00')
                    /Producer     (By hand)
                    /Subject      (test file)
                    >>",
                expected: true,
                expected_result: Some(Dictionary::from([
                    (
                        "CreationDate".to_string(),
                        Object::String(PdfString::Literal("D:20211230134641+11'00'".to_string())),
                    ),
                    (
                        "Creator".to_string(),
                        Object::String(PdfString::Literal("By hand".to_string())),
                    ),
                    (
                        "ModDate".to_string(),
                        Object::String(PdfString::Literal("D:20211230134824+11'00'".to_string())),
                    ),
                    (
                        "Producer".to_string(),
                        Object::String(PdfString::Literal("By hand".to_string())),
                    ),
                    (
                        "Subject".to_string(),
                        Object::String(PdfString::Literal("test file".to_string())),
                    ),
                ])),
                expected_remainder: Some(b""),
            },
            // Invalid dictionaries
            TestCase {
                name: "invalid unclosed dictionary",
                input: b"<< /Key 42",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid content before dictionary",
                input: b"prefix<< /Key 42 >>",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid missing value",
                input: b"<< /Key >>",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
        ];

        for case in &test_cases {
            let result = dictionary(case.input);
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
                    *case.expected_result.as_ref().unwrap(),
                    "Test '{}' failed: expected result: {:?}, got: {:?}",
                    case.name,
                    case.expected_result,
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
