use nom::{
    AsChar, IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_while, take_while_m_n, take_while1},
    combinator::{recognize, value},
    multi::fold,
    sequence::{delimited, preceded},
};

use crate::{parser::whitespace::is_whitespace, types::PdfString};

// NOTE: In most cases here `str::from_utf8` can be safely replaced with unsafe variant

/// Parses a PDF string, which can be a literal or hexadecimal string.
///
/// Literal strings are enclosed in parentheses `(` `)` and may contain escape sequences.
/// Hexadecimal strings are enclosed in angle brackets `<` `>` and consist of hex digits.
pub fn pdf_string(input: &[u8]) -> IResult<&[u8], PdfString> {
    alt((literal_string, hexadecimal_string)).parse(input)
}

/// Parses a literal string enclosed in parentheses, handling escape sequences and balanced parentheses.
fn literal_string(input: &[u8]) -> IResult<&[u8], PdfString> {
    #[derive(Debug)]
    enum Fragment<'a> {
        Literal(&'a [u8]),
        EscapedChar(u8),
        Whitespace,
        InnerString(&'a [u8]),
    }

    let whitespace = preceded(tag("\\"), take_while1(is_whitespace)).map(|_| Fragment::Whitespace);

    let octal_char = take_while_m_n(1, 3, |c| matches!(c, b'0'..=b'7'))
        .map_res(|res| u8::from_str_radix(str::from_utf8(res).unwrap(), 8));

    let escaped_char = preceded(
        tag("\\"),
        alt((
            value(b'\n', tag("n")),
            value(b'\r', tag("r")),
            value(b'\t', tag("t")),
            value(b'\x08', tag("b")),
            value(b'\x0C', tag("f")),
            value(b'(', tag("(")),
            value(b')', tag(")")),
            value(b'\\', tag("\\")),
            octal_char,
        )),
    )
    .map(Fragment::EscapedChar);

    let literal = take_while1(|c| !matches!(c, b'\\' | b'(' | b')')).map(Fragment::Literal);

    let content = alt((
        literal,
        escaped_char,
        whitespace,
        recognize(literal_string).map(Fragment::InnerString),
    ));

    let final_str = fold(
        0..,
        content,
        std::string::String::new,
        |mut data, fragment| {
            match fragment {
                Fragment::Literal(chunk) => data.push_str(str::from_utf8(chunk).unwrap()),
                Fragment::EscapedChar(c) => data.push(c as char),
                Fragment::Whitespace => {}
                Fragment::InnerString(inner) => data.push_str(str::from_utf8(inner).unwrap()),
            }
            data
        },
    );

    delimited(tag("("), final_str, tag(")"))
        .map(PdfString::Literal)
        .parse(input)
}

/// Parses a hexadecimal string enclosed in angle brackets, ignoring non-hex characters.
fn hexadecimal_string(input: &[u8]) -> IResult<&[u8], PdfString> {
    let parse_hex_content = take_while(|c: u8| c != b'>' && (c.is_hex_digit() || is_whitespace(c)))
        .map(|content: &[u8]| {
            content
                .iter()
                .filter(|c| !is_whitespace(**c))
                .copied()
                .collect::<Vec<_>>()
                .chunks(2)
                .map(|chunk| {
                    if chunk.len() == 1 {
                        let mut last = chunk.to_vec();
                        last.push(b'0');

                        return u8::from_str_radix(str::from_utf8(last.as_slice()).unwrap(), 16)
                            .unwrap();
                    }
                    u8::from_str_radix(str::from_utf8(chunk).unwrap(), 16).unwrap()
                })
                .collect::<Vec<_>>()
        });

    delimited(tag("<"), parse_hex_content, tag(">"))
        .map(PdfString::Hexadecimal)
        .parse(input)
}

#[cfg(test)]
mod test {
    use nom::error::dbg_dmp;

    use super::*;

    #[test]
    fn test_string_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            expected: bool,
            expected_result: Option<PdfString>,
            expected_remainder: Option<&'static [u8]>,
        }

        let test_cases = [
            // Valid literal strings
            TestCase {
                name: "valid simple literal string",
                input: b"(hello)",
                expected: true,
                expected_result: Some(PdfString::Literal("hello".to_string())),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid literal with escape sequences",
                input: b"(hello\\nworld\\r\\t\\b\\f\\(\\)\\\\\\12)",
                expected: true,
                expected_result: Some(PdfString::Literal(
                    "hello\nworld\r\t\x08\x0C()\\\n".to_string(),
                )),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid literal with whitespace escape",
                input: b"(hello\\ world)",
                expected: true,
                expected_result: Some(PdfString::Literal("helloworld".to_string())),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid literal with nested parentheses",
                input: b"(hello (nested) world)",
                expected: true,
                expected_result: Some(PdfString::Literal("hello (nested) world".to_string())),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid literal with octal escape",
                input: b"(\\101)",
                expected: true,
                expected_result: Some(PdfString::Literal("A".to_string())),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid literal with multiple octal escapes",
                input: b"(\\101\\102\\103)",
                expected: true,
                expected_result: Some(PdfString::Literal("ABC".to_string())),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid literal with mixed content",
                input: b"(Mix\\055ed\\040Content)",
                expected: true,
                expected_result: Some(PdfString::Literal("Mix-ed Content".to_string())),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid empty literal string",
                input: b"()",
                expected: true,
                expected_result: Some(PdfString::Literal("".to_string())),
                expected_remainder: Some(b""),
            },
            // Valid hexadecimal strings
            TestCase {
                name: "valid simple hex string",
                input: b"<48656C6C6F>",
                expected: true,
                expected_result: Some(PdfString::Hexadecimal(vec![0x48, 0x65, 0x6C, 0x6C, 0x6F])),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid hex with whitespace",
                input: b"<48 65 6C 6C 6F>",
                expected: true,
                expected_result: Some(PdfString::Hexadecimal(vec![0x48, 0x65, 0x6C, 0x6C, 0x6F])),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid hex with odd digits",
                input: b"<4>",
                expected: true,
                expected_result: Some(PdfString::Hexadecimal(vec![0x40])),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid hex with multiple odd digits",
                input: b"<41424>",
                expected: true,
                expected_result: Some(PdfString::Hexadecimal(vec![0x41, 0x42, 0x40])),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid empty hex string",
                input: b"<>",
                expected: true,
                expected_result: Some(PdfString::Hexadecimal(vec![])),
                expected_remainder: Some(b""),
            },
            TestCase {
                name: "valid hex with mixed case",
                input: b"<4a6B>",
                expected: true,
                expected_result: Some(PdfString::Hexadecimal(vec![0x4A, 0x6B])),
                expected_remainder: Some(b""),
            },
            // Valid with remainder
            TestCase {
                name: "literal with remainder",
                input: b"(hello)world",
                expected: true,
                expected_result: Some(PdfString::Literal("hello".to_string())),
                expected_remainder: Some(b"world"),
            },
            TestCase {
                name: "hex with remainder",
                input: b"<68656C6C6F>world",
                expected: true,
                expected_result: Some(PdfString::Hexadecimal(vec![0x68, 0x65, 0x6C, 0x6C, 0x6F])),
                expected_remainder: Some(b"world"),
            },
            // Invalid cases
            TestCase {
                name: "invalid literal unclosed",
                input: b"(hello",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid hex unclosed",
                input: b"<68656C6C6F",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid escape sequence",
                input: b"(hello\\xworld)",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid octal escape",
                input: b"(\\888)",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid hex characters",
                input: b"<GAG>",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "unbalanced parentheses",
                input: b"(hello(world)",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
        ];

        for case in &test_cases {
            let result = dbg_dmp(pdf_string, "pdf_string").parse(case.input);
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
