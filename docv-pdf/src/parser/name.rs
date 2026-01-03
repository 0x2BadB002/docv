use nom::{
    AsChar, IResult, Parser,
    bytes::complete::{tag, take_while, take_while_m_n},
    error::{Error, ErrorKind},
    multi::many0,
    sequence::preceded,
};

use crate::{
    parser::whitespace::{is_delimiter, is_whitespace},
    types::Name,
};

/// Parses a PDF name from a byte slice as defined in the PDF 2.0 standard.
///
/// PDF names start with a slash (`/`) and may contain escaped characters using `#` followed by
/// two hexadecimal digits. Unescaped characters are treated as-is, and the result is a string
/// that includes all decoded characters.
pub fn name(input: &[u8]) -> IResult<&[u8], Name> {
    fn is_regular_symbol(c: u8) -> bool {
        c != b'#' && (b'!'..=b'~').contains(&c) && !is_delimiter(c) && !is_whitespace(c)
    }

    let sym_code_parser = preceded(tag("#"), take_while_m_n(2, 2, |c: u8| c.is_hex_digit()))
        .map(|code| u8::from_str_radix(str::from_utf8(code).unwrap(), 16).unwrap());

    let name = (
        take_while(is_regular_symbol),
        many0((sym_code_parser, take_while(is_regular_symbol))),
    )
        .map_res(|(syms, res)| {
            let capacity = res.iter().map(|(_, codes)| 1 + codes.len()).sum();
            let mut result = syms.to_vec();
            result.reserve_exact(capacity);

            for (sym, codes) in res.iter() {
                result.push(*sym);
                result.extend_from_slice(codes);
            }

            String::from_utf8(result).map_err(|_| Error::new(input, ErrorKind::Fail))
        })
        .map(|res| Name::from(res));

    preceded(tag("/"), name).parse(input)
}

#[cfg(test)]
mod test {
    use super::*;

    use nom::error::dbg_dmp;

    #[test]
    fn test_name_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_result: Option<String>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            TestCase {
                name: "valid name 'Name'",
                input: b"/Name",
                expected: true,
                expected_result: Some("Name".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid name with escape 'A#B'",
                input: b"/A#23B",
                expected: true,
                expected_result: Some("A#B".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid name with multiple escapes",
                input: b"/A#20#21",
                expected: true,
                expected_result: Some("A !".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid name with multiple escapes without regular symbols",
                input: b"/#20#21",
                expected: true,
                expected_result: Some(" !".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "standard invalid null escape",
                input: b"/#00",
                expected: true,
                expected_result: Some("\0".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "empty name",
                input: b"/",
                expected: true,
                expected_result: Some("".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "name with delimiter",
                input: b"/Test(ing",
                expected: true,
                expected_result: Some("Test".to_string()),
                expected_remainder: Some(b"(ing"),
            },
            TestCase {
                name: "name with escaped delimiter",
                input: b"/Test#28ing",
                expected: true,
                expected_result: Some("Test(ing".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "name with whitespace",
                input: b"/Test ing",
                expected: true,
                expected_result: Some("Test".to_string()),
                expected_remainder: Some(b" ing"),
            },
            TestCase {
                name: "name with escaped whitespace",
                input: b"/Test#20ing",
                expected: true,
                expected_result: Some("Test ing".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "name with residual text",
                input: b"/Name123abc",
                expected: true,
                expected_result: Some("Name123abc".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "name with various characters",
                input: b"/A;Name_With-Various***Characters?",
                expected: true,
                expected_result: Some("A;Name_With-Various***Characters?".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "name '1.2'",
                input: b"/1.2",
                expected: true,
                expected_result: Some("1.2".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "name '$$'",
                input: b"/$$",
                expected: true,
                expected_result: Some("$$".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "name '@pattern'",
                input: b"/@pattern",
                expected: true,
                expected_result: Some("@pattern".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "name '.notdef'",
                input: b"/.notdef",
                expected: true,
                expected_result: Some(".notdef".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "name with paired parentheses",
                input: b"/paired#28#29parentheses",
                expected: true,
                expected_result: Some("paired()parentheses".to_string()),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "name with '_' and code",
                input: b"/The_Key_of_F#23_Minor",
                expected: true,
                expected_result: Some("The_Key_of_F#_Minor".to_string()),
                expected_remainder: Some(b""),
            },
        ];

        for case in &test_cases {
            let result = dbg_dmp(name, "name").parse(&case.input);
            assert_eq!(
                result.is_ok(),
                case.expected,
                "Test '{}' failed: expected success: {}, got: {:?}",
                case.name,
                case.expected,
                result
            );

            if case.expected {
                let (actual_remainder, result_str) = result.unwrap();
                let result_str = result_str.to_string();

                assert_eq!(
                    &result_str,
                    case.expected_result.as_ref().unwrap(),
                    "Test '{}' failed: expected result: {:?}, got: {:?}",
                    case.name,
                    case.expected_result,
                    result_str
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
