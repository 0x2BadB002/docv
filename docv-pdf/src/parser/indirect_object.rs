use nom::{
    IResult, ParseTo, Parser,
    branch::alt,
    bytes::complete::tag,
    character::complete::digit1,
    multi::many0,
    sequence::{delimited, terminated},
};

use crate::{
    parser::{
        object::object,
        whitespace::{comment, eol, whitespace},
    },
    types::{IndirectObject, IndirectReference},
};

/// Parses a PDF indirect object from the input.
///
/// # Example
/// 12 0 obj
///     (Hello, World!)
/// endobj
///
/// # Arguments
/// * `input` - Byte slice to parse
///
/// # Returns
/// `IResult` containing:
/// - Remaining input after parsing
/// - Parsed `IndirectObject` on success
pub fn indirect_object(input: &[u8]) -> IResult<&[u8], IndirectObject> {
    let id =
        terminated(digit1, many0(alt((whitespace, comment, eol)))).map_opt(|res| res.parse_to());
    let gen_id =
        terminated(digit1, many0(alt((whitespace, comment, eol)))).map_opt(|res| res.parse_to());

    let contents = delimited(
        many0(alt((whitespace, comment, eol))),
        object,
        many0(alt((whitespace, comment, eol))),
    );

    (id, gen_id, delimited(tag("obj"), contents, tag("endobj")))
        .map(|(id, gen_id, object)| IndirectObject::new(id, gen_id, object))
        .parse(input)
}

/// Parses a PDF indirect object reference from the input.
///
/// # Example
/// 12 0 R
///
/// # Arguments
/// * `input` - Byte slice to parse
///
/// # Returns
/// `IResult` containing:
/// - Remaining input after parsing
/// - Parsed [`IndirectReference`] on success
pub fn indirect_reference(input: &[u8]) -> IResult<&[u8], IndirectReference> {
    let id = terminated(digit1, many0(alt((whitespace, comment, eol))))
        .map_res(|res| str::from_utf8(res).unwrap().parse());
    let gen_id = terminated(digit1, many0(alt((whitespace, comment, eol))))
        .map_res(|res| str::from_utf8(res).unwrap().parse());

    terminated((id, gen_id), tag("R"))
        .map(|(id, gen_id)| IndirectReference { id, gen_id })
        .parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Numeric, Object};
    use nom::error::dbg_dmp;

    #[test]
    fn test_indirect_object_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_id: Option<usize>,
            expected_gen_id: Option<usize>,
            expected_object: Option<Object>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            // Valid indirect objects
            TestCase {
                name: "minimal valid indirect object",
                input: b"1 0 obj true endobj",
                expected: true,
                expected_id: Some(1),
                expected_gen_id: Some(0),
                expected_object: Some(Object::Boolean(true)),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "indirect object with string content",
                input: b"2 0 obj (Hello) endobj",
                expected: true,
                expected_id: Some(2),
                expected_gen_id: Some(0),
                expected_object: Some(Object::String(crate::types::PdfString::Literal(
                    String::from_utf8(b"Hello".to_vec()).unwrap(),
                ))),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "indirect object with numeric content",
                input: b"3 1 obj 42 endobj",
                expected: true,
                expected_id: Some(3),
                expected_gen_id: Some(1),
                expected_object: Some(Object::Numeric(Numeric::Integer(42))),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "indirect object with comments and whitespace",
                input: b"4\t0%comment\nobj%comment\n[1 2 3]%comment\nendobj",
                expected: true,
                expected_id: Some(4),
                expected_gen_id: Some(0),
                expected_object: Some(Object::Array(
                    vec![
                        Object::Numeric(Numeric::Integer(1)),
                        Object::Numeric(Numeric::Integer(2)),
                        Object::Numeric(Numeric::Integer(3)),
                    ]
                    .into(),
                )),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "indirect object with trailing content",
                input: b"5 0 obj null endobjrest",
                expected: true,
                expected_id: Some(5),
                expected_gen_id: Some(0),
                expected_object: Some(Object::Null),
                expected_remainder: Some(b"rest"),
            },
            // Invalid indirect objects
            TestCase {
                name: "missing endobj",
                input: b"6 0 obj true",
                expected: false,
                expected_id: None,
                expected_gen_id: None,
                expected_object: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid generation number",
                input: b"7 x obj true endobj",
                expected: false,
                expected_id: None,
                expected_gen_id: None,
                expected_object: None,
                expected_remainder: None,
            },
            TestCase {
                name: "missing object content",
                input: b"8 0 obj endobj",
                expected: false,
                expected_id: None,
                expected_gen_id: None,
                expected_object: None,
                expected_remainder: None,
            },
        ];

        for case in &test_cases {
            let result = dbg_dmp(indirect_object, "indirect_object").parse(case.input);
            assert_eq!(
                result.is_ok(),
                case.expected,
                "Test '{}' failed: expected success: {}",
                case.name,
                case.expected
            );

            if let Ok((actual_remainder, actual)) = result {
                let expected_object = case.expected_object.as_ref().unwrap();
                assert_eq!(
                    actual.id,
                    case.expected_id.unwrap(),
                    "Test '{}' failed: expected id: {:?}, got: {:?}",
                    case.name,
                    case.expected_id,
                    actual.id
                );
                assert_eq!(
                    actual.gen_id,
                    case.expected_gen_id.unwrap(),
                    "Test '{}' failed: expected gen_id: {:?}, got: {:?}",
                    case.name,
                    case.expected_gen_id,
                    actual.gen_id
                );
                assert_eq!(
                    *actual.get_object(),
                    *expected_object,
                    "Test '{}' failed: expected object: {:?}, got: {:?}",
                    case.name,
                    case.expected_object,
                    *actual.get_object()
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
    fn test_indirect_reference_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_id: Option<usize>,
            expected_gen_id: Option<usize>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            // Valid references
            TestCase {
                name: "minimal valid reference",
                input: b"1 0 R",
                expected: true,
                expected_id: Some(1),
                expected_gen_id: Some(0),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "reference with whitespace",
                input: b"2\t0  R",
                expected: true,
                expected_id: Some(2),
                expected_gen_id: Some(0),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "reference with comments",
                input: b"3%comment\n0%comment\nR",
                expected: true,
                expected_id: Some(3),
                expected_gen_id: Some(0),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "reference with trailing content",
                input: b"4 0 Rrest",
                expected: true,
                expected_id: Some(4),
                expected_gen_id: Some(0),
                expected_remainder: Some(b"rest"),
            },
            // Invalid references
            TestCase {
                name: "missing R",
                input: b"5 0",
                expected: false,
                expected_id: None,
                expected_gen_id: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid generation number",
                input: b"6 x R",
                expected: false,
                expected_id: None,
                expected_gen_id: None,
                expected_remainder: None,
            },
            TestCase {
                name: "missing generation number",
                input: b"7 R",
                expected: false,
                expected_id: None,
                expected_gen_id: None,
                expected_remainder: None,
            },
        ];

        for case in &test_cases {
            let result = dbg_dmp(indirect_reference, "indirect_reference").parse(case.input);
            assert_eq!(
                result.is_ok(),
                case.expected,
                "Test '{}' failed: expected success: {}",
                case.name,
                case.expected
            );

            if let Ok((actual_remainder, actual)) = result {
                assert_eq!(
                    actual.id,
                    case.expected_id.unwrap(),
                    "Test '{}' failed: expected id: {:?}, got: {:?}",
                    case.name,
                    case.expected_id,
                    actual.id
                );
                assert_eq!(
                    actual.gen_id,
                    case.expected_gen_id.unwrap(),
                    "Test '{}' failed: expected gen_id: {:?}, got: {:?}",
                    case.name,
                    case.expected_gen_id,
                    actual.gen_id
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
