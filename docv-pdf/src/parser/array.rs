use nom::{IResult, Parser, branch::alt, bytes::complete::tag, multi::many0, sequence::delimited};

use crate::{
    parser::{
        object::object,
        whitespace::{comment, eol, whitespace},
    },
    types::Array,
};

/// Parses a PDF array from the input.
///
/// PDF arrays are enclosed in square brackets and contain zero or more objects.
/// Allows optional whitespace, comments, and end-of-line markers between elements.
///
/// # Example
/// [1 (two) /three]  // Array with three elements
///
/// # Arguments
/// * `input` - Byte slice to parse
///
/// # Returns
/// `IResult` containing remaining input and parsed `Vec<Object>` on success
pub fn array(input: &[u8]) -> IResult<&[u8], Array> {
    let contents = many0(delimited(
        many0(alt((whitespace, comment, eol))),
        object,
        many0(alt((whitespace, comment, eol))),
    ));

    delimited(tag("["), contents, tag("]"))
        .map(Array::from)
        .parse(input)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::{Numeric, Object, PdfString};
    use nom::error::dbg_dmp;

    #[test]
    fn test_array_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_result: Option<Array>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            // Valid arrays
            TestCase {
                name: "valid empty array",
                input: b"[]",
                expected: true,
                expected_result: Some(vec![].into()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid array with integers",
                input: b"[1 2 3]",
                expected: true,
                expected_result: Some(
                    vec![
                        Object::Numeric(Numeric::Integer(1)),
                        Object::Numeric(Numeric::Integer(2)),
                        Object::Numeric(Numeric::Integer(3)),
                    ]
                    .into(),
                ),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid array with mixed types",
                input: b"[1 (two) /three]",
                expected: true,
                expected_result: Some(
                    vec![
                        Object::Numeric(Numeric::Integer(1)),
                        Object::String(PdfString::Literal("two".to_string())),
                        Object::Name("three".into()),
                    ]
                    .into(),
                ),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid array with comments and whitespace",
                input: b"[ % comment\n1 % another\n (two) \t /three \n]",
                expected: true,
                expected_result: Some(
                    vec![
                        Object::Numeric(Numeric::Integer(1)),
                        Object::String(PdfString::Literal("two".to_string())),
                        Object::Name("three".into()),
                    ]
                    .into(),
                ),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid array with nested arrays",
                input: b"[1 [2 3] (four)]",
                expected: true,
                expected_result: Some(
                    vec![
                        Object::Numeric(Numeric::Integer(1)),
                        Object::Array(
                            vec![
                                Object::Numeric(Numeric::Integer(2)),
                                Object::Numeric(Numeric::Integer(3)),
                            ]
                            .into(),
                        ),
                        Object::String(PdfString::Literal("four".to_string())),
                    ]
                    .into(),
                ),
                expected_remainder: Some(b""),
            },
            // Valid with remainder
            TestCase {
                name: "array with remainder",
                input: b"[1 2 3]rest",
                expected: true,
                expected_result: Some(
                    vec![
                        Object::Numeric(Numeric::Integer(1)),
                        Object::Numeric(Numeric::Integer(2)),
                        Object::Numeric(Numeric::Integer(3)),
                    ]
                    .into(),
                ),
                expected_remainder: Some(b"rest"),
            },
            // Invalid arrays
            TestCase {
                name: "invalid unclosed array",
                input: b"[1 2 3",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid content before array",
                input: b"prefix[1]",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
        ];

        for case in &test_cases {
            let result = dbg_dmp(array, "pdf_array").parse(case.input);
            assert_eq!(
                result.is_ok(),
                case.expected,
                "Test '{}' failed: expected success: {}, got: {:?}",
                case.name,
                case.expected,
                result
            );

            if case.expected {
                let (actual_remainder, result_array) = result.unwrap();
                assert_eq!(
                    &result_array,
                    case.expected_result.as_ref().unwrap(),
                    "Test '{}' failed: expected result: {:?}, got: {:?}",
                    case.name,
                    case.expected_result,
                    result_array
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
