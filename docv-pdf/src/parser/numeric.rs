use nom::{
    IResult, ParseTo, Parser,
    bytes::complete::tag,
    character::complete::{digit0, digit1, one_of},
    combinator::{opt, recognize},
    sequence::preceded,
};

use crate::types::Numeric;

/// Parses a numeric value from a byte slice as defined in the PDF 2.0 standard.
///
/// This parser supports both integer and real number formats, including negative values
/// and optional decimal points. It consumes input until the end of the number.
pub fn numeric(input: &[u8]) -> IResult<&[u8], Numeric> {
    let (remaining, num_str) = recognize(preceded(
        opt(one_of("+-")),
        (opt((digit0, tag("."))), digit1),
    ))
    .parse(input)?;

    if num_str.contains(&b'.') {
        let num = num_str.parse_to().ok_or_else(|| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Float))
        })?;
        Ok((remaining, Numeric::Real(num)))
    } else {
        let num = num_str.parse_to().ok_or_else(|| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Digit))
        })?;
        Ok((remaining, Numeric::Integer(num)))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_numeric_parser() {
        #[derive(Debug, PartialEq)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_result: Option<Numeric>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            TestCase {
                name: "valid integer '123'",
                input: b"123",
                expected: true,
                expected_result: Some(Numeric::Integer(123)),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid negative integer '-456'",
                input: b"-456",
                expected: true,
                expected_result: Some(Numeric::Integer(-456)),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid positive integer '+789'",
                input: b"+789",
                expected: true,
                expected_result: Some(Numeric::Integer(789)),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid real number '123.45'",
                input: b"123.45",
                expected: true,
                expected_result: Some(Numeric::Real(123.45)),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid real number '.45'",
                input: b".45",
                expected: true,
                expected_result: Some(Numeric::Real(0.45)),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid negative real number '-678.90'",
                input: b"-678.90",
                expected: true,
                expected_result: Some(Numeric::Real(-678.9)),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid positive real number '+12.34'",
                input: b"+12.34",
                expected: true,
                expected_result: Some(Numeric::Real(12.34)),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid positive real number '+.34'",
                input: b"+.34",
                expected: true,
                expected_result: Some(Numeric::Real(0.34)),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "invalid 'abc'",
                input: b"abc",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "integer with some text '12a3'",
                input: b"12a3",
                expected: true,
                expected_result: Some(Numeric::Integer(12)),
                expected_remainder: Some(b"a3"),
            },
            TestCase {
                name: "invalid empty input",
                input: b"",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid '12.' (no digits after decimal)",
                input: b"12.",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "valid with residual text '123.45a'",
                input: b"123.45a",
                expected: true,
                expected_result: Some(Numeric::Real(123.45)),
                expected_remainder: Some(b"a"),
            },
            TestCase {
                name: "valid with residual text '123.45.67' (extra decimal)",
                input: b"123.45.67",
                expected: true,
                expected_result: Some(Numeric::Real(123.45)),
                expected_remainder: Some(b".67"),
            },
        ];

        for case in &test_cases {
            let result = numeric(case.input);
            let success = result.is_ok();
            assert_eq!(
                success, case.expected,
                "Test '{}' failed: expected success: {}, got: {}",
                case.name, case.expected, success
            );

            if case.expected {
                let (actual_remainder, result) = match result {
                    Ok((rem, res)) => (rem, res),
                    Err(e) => panic!(
                        "Parsing failed for test '{}', input: {:#?}, error: {e:?}",
                        case.name, case.input
                    ),
                };
                let expected_result = case.expected_result.as_ref().unwrap();
                assert_eq!(
                    result, *expected_result,
                    "Test '{}' failed: expected result: {:?}, got: {:?}",
                    case.name, *expected_result, result
                );
                assert_eq!(
                    actual_remainder,
                    case.expected_remainder.unwrap(),
                    "Test '{}' failed: expected remainder: {:?}, got: {:?}",
                    case.name,
                    case.expected_remainder.unwrap(),
                    actual_remainder
                );
            }
        }
    }
}
