use nom::{
    IResult, Parser,
    branch::alt,
    bytes::{
        complete::{tag, take_while1},
        is_not,
    },
    combinator::value,
    sequence::delimited,
};

/// Checks if a byte represents a PDF whitespace character as defined by the PDF2.0 standard.
///
/// Whitespace includes null (0x00), tab (0x09), LF (0x0A), FF (0x0C), CR (0x0D), and space (0x20).
/// Returns `true` if the character is a whitespace, `false` otherwise.
pub fn is_whitespace(c: u8) -> bool {
    matches!(c, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
}

/// Checks if a byte represents a PDF delimiter character as defined by the PDF2.0 standard.
///
/// Delimiters include `(` (0x28), `)` (0x29), `<` (0x3c), `>` (0x3e), `[` (0x5b), `]` (0x5d),
/// `{` (0x7b), `}` (0x7d), `/` (0x2f), and `%` (0x25).
/// Returns `true` if the character is a delimiter, `false` otherwise.
pub fn is_delimiter(c: u8) -> bool {
    matches!(
        c,
        0x28 | 0x29 | 0x3c | 0x3e | 0x5b | 0x5d | 0x7b | 0x7d | 0x2f | 0x25
    )
}

/// Parses one or more PDF whitespace characters (null, tab, LF, FF, CR, space).
///
/// Returns `Ok` with an empty tuple if whitespace is found, otherwise `Err`.
/// Consumes the entire input if it consists solely of whitespace.
pub fn whitespace(input: &[u8]) -> IResult<&[u8], ()> {
    value((), take_while1(is_whitespace)).parse(input)
}

/// Parses PDF end-of-line markers (CR, LF, or CR followed by LF).
///
/// Returns `Ok` with an empty tuple if an EOL is found, otherwise `Err`.
/// Consumes the matched EOL sequence and returns the remaining input.
pub fn eol(input: &[u8]) -> IResult<&[u8], ()> {
    value((), alt((tag("\x0D\x0A"), tag("\x0D"), tag("\x0A")))).parse(input)
}

/// Parses PDF comments starting with `%` and ending at the first EOL.
///
/// Returns `Ok` with an empty tuple if a comment is found, otherwise `Err`.
/// Consumes the `%` and comment content, leaving the EOL character(s) in the remainder.
pub fn comment(input: &[u8]) -> IResult<&[u8], ()> {
    value((), delimited(tag("%"), is_not("\x0D\x0A"), eol)).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitespace_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_remainder: &'static [u8],
        }

        let test_cases = [
            TestCase {
                name: "empty input",
                input: b"",
                expected: false,
                expected_remainder: b"",
            },
            TestCase {
                name: "single space",
                input: b" ",
                expected: true,
                expected_remainder: b"",
            },
            TestCase {
                name: "multiple spaces",
                input: b"  ",
                expected: true,
                expected_remainder: b"",
            },
            TestCase {
                name: "whitespace followed by text",
                input: b"  abc",
                expected: true,
                expected_remainder: b"abc",
            },
            TestCase {
                name: "text with no whitespace",
                input: b"abc",
                expected: false,
                expected_remainder: b"abc",
            },
            TestCase {
                name: "text with different whitespace chars",
                input: b" \t\nabc",
                expected: true,
                expected_remainder: b"abc",
            },
            TestCase {
                name: "mixed whitespace and text",
                input: b" \r a b c",
                expected: true,
                expected_remainder: b"a b c",
            },
        ];

        for case in &test_cases {
            let result = whitespace(case.input);
            let success = result.is_ok();
            assert_eq!(
                success, case.expected,
                "Test '{}' failed: expected success: {}, got: {}",
                case.name, case.expected, success
            );

            if case.expected {
                let actual_remainder = match result {
                    Ok((rem, _)) => rem,
                    Err(e) => panic!(
                        "Parsing failed for test '{}', input: {:#?}, error: {e:?}",
                        case.name, case.input
                    ),
                };

                assert_eq!(
                    actual_remainder, case.expected_remainder,
                    "Test '{}' failed: expected remainder: {:#?}, got: {:#?}",
                    case.name, case.expected_remainder, actual_remainder
                );
            }
        }
    }

    #[test]
    fn test_eol_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_remainder: &'static [u8],
        }

        let test_cases = [
            TestCase {
                name: "CRLF",
                input: b"\r\n",
                expected: true,
                expected_remainder: b"",
            },
            TestCase {
                name: "CR only",
                input: b"\r",
                expected: true,
                expected_remainder: b"",
            },
            TestCase {
                name: "LF only",
                input: b"\n",
                expected: true,
                expected_remainder: b"",
            },
            TestCase {
                name: "double CR",
                input: b"\r\r",
                expected: true,
                expected_remainder: b"\r",
            },
            TestCase {
                name: "LF followed by CR",
                input: b"\n\r",
                expected: true,
                expected_remainder: b"\r",
            },
            TestCase {
                name: "empty input",
                input: b"",
                expected: false,
                expected_remainder: b"",
            },
            TestCase {
                name: "CRLF followed by CR",
                input: b"\r\n\x0D",
                expected: true,
                expected_remainder: b"\x0D",
            },
            TestCase {
                name: "CR followed by CRLF",
                input: b"\r\x0D\x0A",
                expected: true,
                expected_remainder: b"\r\n",
            },
            TestCase {
                name: "CR followed by LF",
                input: b"\r\x0A",
                expected: true,
                expected_remainder: b"",
            },
            TestCase {
                name: "LF followed by CR",
                input: b"\x0A\r",
                expected: true,
                expected_remainder: b"\r",
            },
        ];

        for case in &test_cases {
            let result = eol(case.input);
            let success = result.is_ok();
            assert_eq!(
                success, case.expected,
                "Test '{}' failed: expected success: {}, got: {}",
                case.name, case.expected, success
            );

            if case.expected {
                let actual_remainder = match result {
                    Ok((rem, _)) => rem,
                    Err(e) => panic!(
                        "Parsing failed for test '{}', input: {:#?}, error: {e:?}",
                        case.name, case.input
                    ),
                };
                assert_eq!(
                    actual_remainder, case.expected_remainder,
                    "Test '{}' failed: expected remainder: {:#?}, got: {:#?}",
                    case.name, case.expected_remainder, actual_remainder
                );
            }
        }
    }

    #[test]
    fn test_comment_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_remainder: &'static [u8],
        }

        let test_cases = [
            TestCase {
                name: "empty comment",
                input: b"",
                expected: false,
                expected_remainder: b"",
            },
            TestCase {
                name: "comment with text",
                input: b"% this is a comment\n",
                expected: true,
                expected_remainder: b"",
            },
            TestCase {
                name: "comment with trailing text",
                input: b"% comment abc123 \n",
                expected: true,
                expected_remainder: b"",
            },
            TestCase {
                name: "no comment character",
                input: b"this is not a comment",
                expected: false,
                expected_remainder: b"this is not a comment",
            },
            TestCase {
                name: "partial comment",
                input: b"%",
                expected: false,
                expected_remainder: b"%",
            },
            TestCase {
                name: "multiline comment",
                input: b"% line 1\n% line 2",
                expected: true,
                expected_remainder: b"% line 2",
            },
        ];

        for case in &test_cases {
            let result = comment(case.input);
            let success = result.is_ok();
            assert_eq!(
                success, case.expected,
                "Test '{}' failed: expected success: {}, got: {}",
                case.name, case.expected, success
            );

            if case.expected {
                let actual_remainder = match result {
                    Ok((rem, _)) => rem,
                    Err(e) => panic!(
                        "Parsing failed for test '{}', input: {:#?}, error: {e:?}",
                        case.name, case.input
                    ),
                };
                assert_eq!(
                    actual_remainder, case.expected_remainder,
                    "Test '{}' failed: expected remainder: {:#?}, got: {:#?}",
                    case.name, case.expected_remainder, actual_remainder
                );
            }
        }
    }
}
