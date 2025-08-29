use nom::{Finish, IResult, Parser, branch::alt, error::Error};

use crate::{
    parser::{
        array::array,
        boolean::boolean,
        dictionary::dictionary,
        indirect_object::{indirect_object, indirect_reference},
        name::name,
        null::null,
        numeric::numeric,
        stream::stream,
        string::pdf_string,
    },
    types::Object,
};

/// Parses a PDF object from the input.
///
/// Attempts to parse any of the fundamental PDF object types:
/// - Boolean (`true`/`false`)
/// - Numeric (integer or real)
/// - String (literal or hexadecimal)
/// - Name (e.g., `/Foo`)
/// - Null (`null`)
/// - Array (e.g., `[1 2 /Three]`)
/// - Dictionary (e.g., `<< \type \XRef >>`)
/// - Stream (e.g., `<< \type \XObject \Length 10 >>stream ... endstream`)
/// - Indirect Objects (e.g., 1 0 obj ... endobj)
///
/// # Arguments
/// * `input` - Byte slice to parse
///
/// # Returns
/// `IResult` containing remaining input and parsed [`Object`] on success
pub fn read_object(input: &[u8]) -> Result<Object, Error<&[u8]>> {
    let (_, object) = object(input).finish()?;

    Ok(object)
}

/// Parses a PDF object from the input.
///
/// Attempts to parse any of the fundamental PDF object types:
/// - Boolean (`true`/`false`)
/// - Numeric (integer or real)
/// - String (literal or hexadecimal)
/// - Name (e.g., `/Foo`)
/// - Null (`null`)
/// - Array (e.g., `[1 2 /Three]`)
/// - Dictionary (e.g., `<< \type \XRef >>`)
/// - Stream (e.g., `<< \type \XObject \Length 10 >>stream ... endstream`)
/// - Indirect Objects (e.g., 1 0 obj ... endobj)
///
/// # Arguments
/// * `input` - Byte slice to parse
///
/// # Returns
/// `IResult` containing remaining input and parsed `Object` on success
pub fn object(input: &[u8]) -> IResult<&[u8], Object> {
    alt((
        indirect_object.map(Object::IndirectDefinition),
        indirect_reference.map(Object::IndirectReference),
        stream.map(Object::Stream),
        dictionary.map(Object::Dictionary),
        array.map(Object::Array),
        numeric.map(Object::Numeric),
        pdf_string.map(Object::String),
        name.map(Object::Name),
        boolean.map(Object::Boolean),
        null.map(|_| Object::Null),
    ))
    .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Array, Dictionary, IndirectObject, IndirectReference, Numeric, PdfString, Stream,
    };
    use std::{collections::BTreeMap, sync::Arc};

    #[test]
    fn test_object_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_value: Option<Object>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            // Boolean tests
            TestCase {
                name: "boolean true",
                input: b"true",
                expected: true,
                expected_value: Some(Object::Boolean(true)),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "boolean false",
                input: b"false",
                expected: true,
                expected_value: Some(Object::Boolean(false)),
                expected_remainder: Some(b""),
            },
            // Numeric tests
            TestCase {
                name: "integer numeric",
                input: b"123",
                expected: true,
                expected_value: Some(Object::Numeric(Numeric::Integer(123))),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "real numeric",
                input: b"3.14",
                expected: true,
                expected_value: Some(Object::Numeric(Numeric::Real(3.14))),
                expected_remainder: Some(b""),
            },
            // String tests
            TestCase {
                name: "literal string",
                input: b"(Hello World)",
                expected: true,
                expected_value: Some(Object::String(PdfString::Literal(
                    "Hello World".to_string(),
                ))),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "hex string",
                input: b"<48656C6C6F>",
                expected: true,
                expected_value: Some(Object::String(PdfString::Hexadecimal(vec![
                    0x48, 0x65, 0x6C, 0x6C, 0x6F,
                ]))),
                expected_remainder: Some(b""),
            },
            // Name tests
            TestCase {
                name: "name object",
                input: b"/FontName",
                expected: true,
                expected_value: Some(Object::Name("FontName".to_string())),
                expected_remainder: Some(b""),
            },
            // Null test
            TestCase {
                name: "null object",
                input: b"null",
                expected: true,
                expected_value: Some(Object::Null),
                expected_remainder: Some(b""),
            },
            // Array test
            TestCase {
                name: "array object",
                input: b"[1 2 /Three]",
                expected: true,
                expected_value: Some(Object::Array(
                    vec![
                        Object::Numeric(Numeric::Integer(1)),
                        Object::Numeric(Numeric::Integer(2)),
                        Object::Name("Three".to_string()),
                    ]
                    .into(),
                )),
                expected_remainder: Some(b""),
            },
            // Dictionary test
            TestCase {
                name: "dictionary object",
                input: b"<< /Key (Value) >>",
                expected: true,
                expected_value: Some(Object::Dictionary(Dictionary {
                    records: BTreeMap::from([(
                        "Key".to_string(),
                        Object::String(PdfString::Literal("Value".to_string())),
                    )]),
                })),
                expected_remainder: Some(b""),
            },
            // Indirect reference test
            TestCase {
                name: "indirect reference",
                input: b"1 0 R",
                expected: true,
                expected_value: Some(Object::IndirectReference(IndirectReference {
                    id: 1,
                    gen_id: 0,
                })),
                expected_remainder: Some(b""),
            },
            // Stream test (simplified)
            TestCase {
                name: "stream object",
                input: b"<< /Length 10 >> stream\n0123456789\nendstream",
                expected: true,
                expected_value: Some(Object::Stream(Stream {
                    dictionary: Dictionary::from([(
                        "Length".to_string(),
                        Object::Numeric(Numeric::Integer(10)),
                    )]),
                    data: b"0123456789".to_vec(),
                })),
                expected_remainder: Some(b""),
            },
            // Error cases
            TestCase {
                name: "invalid input",
                input: b"invalid",
                expected: false,
                expected_value: None,
                expected_remainder: None,
            },
        ];

        for case in &test_cases {
            let result = object(case.input);
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
                    case.expected_value.as_ref().unwrap().clone(),
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
}
