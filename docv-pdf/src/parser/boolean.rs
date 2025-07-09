use nom::{IResult, Parser, branch::alt, bytes::complete::tag, combinator::value};

/// Parses a boolean value from a byte slice as defined in the PDF 2.0 standard.
///
/// PDF boolean literals are represented as either "true" or "false".
pub fn boolean(input: &[u8]) -> IResult<&[u8], bool> {
    alt((value(true, tag("true")), value(false, tag("false")))).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boolean_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_result: Option<bool>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            TestCase {
                name: "valid 'true'",
                input: b"true",
                expected: true,
                expected_result: Some(true),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid 'false'",
                input: b"false",
                expected: true,
                expected_result: Some(false),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "invalid 'True'",
                input: b"True",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid empty input",
                input: b"",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid 'Tr'",
                input: b"Tr",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid 'FALSE'",
                input: b"FALSE",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid 'truefalse'",
                input: b"truefalse",
                expected: true,
                expected_result: Some(true),
                expected_remainder: Some(b"false"),
            },
        ];

        for case in &test_cases {
            let result = boolean(case.input);
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
                assert_eq!(
                    result,
                    case.expected_result.unwrap(),
                    "Test '{}' failed: expected result: {:?}, got: {:?}",
                    case.name,
                    case.expected_result,
                    result
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
