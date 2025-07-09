use nom::{IResult, Parser, bytes::complete::tag, combinator::value};

/// Parses a PDF null literal from a byte slice as defined in the PDF 2.0 standard.
///
/// PDF null values are represented by the exact byte sequence "null".
/// Returns a unit type `()` on successful parse to indicate null recognition.
pub fn null(input: &[u8]) -> IResult<&[u8], ()> {
    value((), tag("null")).parse(input)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_null_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            TestCase {
                name: "valid 'null'",
                input: b"null",
                expected: true,
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid 'null' with remainder",
                input: b"nullptr",
                expected: true,
                expected_remainder: Some(b"ptr"),
            },
            TestCase {
                name: "invalid 'nul'",
                input: b"nul",
                expected: false,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid 'Null' (case-sensitive)",
                input: b"Null",
                expected: false,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid empty input",
                input: b"",
                expected: false,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid partial match",
                input: b"nulL",
                expected: false,
                expected_remainder: None,
            },
            TestCase {
                name: "valid null followed by delimiter",
                input: b"null(abc",
                expected: true,
                expected_remainder: Some(b"(abc"),
            },
        ];

        for case in &test_cases {
            let result = null(case.input);
            let success = result.is_ok();
            assert_eq!(
                success, case.expected,
                "Test '{}' failed: expected success: {}, got: {}",
                case.name, case.expected, success
            );

            if case.expected {
                let (actual_remainder, _) = result.unwrap_or_else(|_| {
                    panic!("Parser failed unexpectedly for test case: {}", case.name)
                });
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
