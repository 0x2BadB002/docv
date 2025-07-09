use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::tag,
    multi::{many0, many1},
    sequence::{delimited, preceded},
};

use crate::parser::{
    Object,
    name::name,
    object,
    whitespace::{comment, eol, whitespace},
};

/// Represents a single key-value pair in a PDF dictionary.
///
/// PDF dictionaries consist of name-object pairs where:
/// - `key` is a PDF name (always starts with '/')
/// - `value` is any valid PDF object
#[derive(Debug, PartialEq, Clone)]
pub struct DictionaryRecord {
    pub key: String,
    pub value: Object,
}

/// Parses a PDF dictionary from the input.
///
/// PDF dictionaries are enclosed in double angle brackets (`<<` and `>>`) and contain
/// one or more key-value pairs. Allows optional whitespace, comments, and end-of-line
/// markers between elements.
///
/// # Example
/// ```
/// <<
///     /Key1 42
///     /Key2 (Text value)
///     % This is a comment
///     /Key3 [1 2 3]  % Array value
/// >>
/// ```
///
/// # Arguments
/// * `input` - Byte slice to parse
///
/// # Returns
/// `IResult` containing:
/// - Remaining input after parsing
/// - Iterator over `DictionaryRecord` key-value pairs on success
pub fn dictionary(input: &[u8]) -> IResult<&[u8], impl Iterator<Item = DictionaryRecord>> {
    let key_value = (
        name,
        preceded(many1(alt((whitespace, comment, eol))), object),
    )
        .map(|(key, value)| DictionaryRecord { key, value });

    let contents = many0(delimited(
        many0(alt((whitespace, comment, eol))),
        key_value,
        many0(alt((whitespace, comment, eol))),
    ))
    .map(|res| res.into_iter());

    delimited(tag("<<"), contents, tag(">>")).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Numeric, Object, PdfString};
    use nom::error::dbg_dmp;

    #[test]
    fn test_dictionary_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_result: Option<Vec<DictionaryRecord>>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            // Valid dictionaries
            TestCase {
                name: "valid empty dictionary",
                input: b"<<>>",
                expected: true,
                expected_result: Some(vec![]),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid dictionary with one key-value pair",
                input: b"<< /Key 42 >>",
                expected: true,
                expected_result: Some(vec![DictionaryRecord {
                    key: "Key".to_string(),
                    value: Object::Numeric(Numeric::Integer(42)),
                }]),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid dictionary with multiple key-value pairs",
                input: b"<< /Key1 1 /Key2 (two) /Key3 /three >>",
                expected: true,
                expected_result: Some(vec![
                    DictionaryRecord {
                        key: "Key1".to_string(),
                        value: Object::Numeric(Numeric::Integer(1)),
                    },
                    DictionaryRecord {
                        key: "Key2".to_string(),
                        value: Object::String(PdfString::Literal("two".to_string())),
                    },
                    DictionaryRecord {
                        key: "Key3".to_string(),
                        value: Object::Name("three".to_string()),
                    },
                ]),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid dictionary with comments and whitespace",
                input: b"<< % comment\n /Key1 1 % another\n /Key2 (two) \t /Key3 /three \n>>",
                expected: true,
                expected_result: Some(vec![
                    DictionaryRecord {
                        key: "Key1".to_string(),
                        value: Object::Numeric(Numeric::Integer(1)),
                    },
                    DictionaryRecord {
                        key: "Key2".to_string(),
                        value: Object::String(PdfString::Literal("two".to_string())),
                    },
                    DictionaryRecord {
                        key: "Key3".to_string(),
                        value: Object::Name("three".to_string()),
                    },
                ]),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid dictionary with nested structures",
                input: b"<< /Array [1 2 3] /Nested << /SubKey true >> >>",
                expected: true,
                expected_result: Some(vec![
                    DictionaryRecord {
                        key: "Array".to_string(),
                        value: Object::Array(vec![
                            Object::Numeric(Numeric::Integer(1)),
                            Object::Numeric(Numeric::Integer(2)),
                            Object::Numeric(Numeric::Integer(3)),
                        ]),
                    },
                    DictionaryRecord {
                        key: "Nested".to_string(),
                        value: Object::Dictionary(vec![DictionaryRecord {
                            key: "SubKey".to_string(),
                            value: Object::Boolean(true),
                        }]),
                    },
                ]),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "dictionary with remainder",
                input: b"<< /Key 42 >>rest",
                expected: true,
                expected_result: Some(vec![DictionaryRecord {
                    key: "Key".to_string(),
                    value: Object::Numeric(Numeric::Integer(42)),
                }]),
                expected_remainder: Some(b"rest"),
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
            let result = dbg_dmp(dictionary, "pdf_dictionary").parse(case.input);
            assert_eq!(
                result.is_ok(),
                case.expected,
                "Test '{}' failed: expected success: {}",
                case.name,
                case.expected,
            );

            if case.expected {
                let (actual_remainder, result_iter) = result.unwrap();
                let result_vec: Vec<DictionaryRecord> = result_iter.collect();
                assert_eq!(
                    result_vec,
                    *case.expected_result.as_ref().unwrap(),
                    "Test '{}' failed: expected result: {:?}, got: {:?}",
                    case.name,
                    case.expected_result,
                    result_vec
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
