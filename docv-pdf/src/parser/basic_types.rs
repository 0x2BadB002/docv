use std::io::Read;

use flate2::read::ZlibDecoder;
use pest::iterators::Pair;

use crate::parser::Rule;
use crate::{Error, Result};

#[derive(Default, Debug)]
enum StreamFilterType {
    #[default]
    None,
    FlateDecode,
}

pub fn parse_numeric<T>(object: Pair<Rule>) -> Result<T>
where
    T: std::str::FromStr<Err = std::num::ParseIntError>,
{
    find_child_object(object, Rule::numeric)?
        .as_str()
        .parse()
        .map_err(Error::IntConv)
}

pub fn parse_name(object: Pair<Rule>) -> Result<&str> {
    let name = find_child_object(object, Rule::name)?;

    Ok(name.into_inner().next().map_or("", |data| data.as_str()))
}

pub fn parse_string(object: Pair<Rule>) -> Result<&str> {
    Ok(find_child_object(object, Rule::string)?
        .into_inner()
        .next()
        .ok_or(Error::InvalidString)?
        .into_inner()
        .next()
        .map_or("", |data| data.as_str()))
}

pub fn parse_literal_string(object: Pair<Rule>) -> Result<&str> {
    Ok(find_child_object(object, Rule::literal_string)?
        .into_inner()
        .next()
        .map_or("", |data| data.as_str()))
}

pub fn parse_hex_string(object: Pair<Rule>) -> Result<Vec<u8>> {
    let mut hex_data = find_child_object(object, Rule::hex_string)?
        .into_inner()
        .next()
        .map_or("", |data| data.as_str())
        .to_string();

    // If string len is uneven assume that "0" is at the end.
    // Part of the standard.
    if hex_data.len() % 2 != 0 {
        hex_data += "0";
    }

    hex_data
        .as_bytes()
        .chunks_exact(2)
        .map(|chunk| {
            chunk
                .iter()
                .map(|c| match c {
                    b'0'..=b'9' => Ok(c - b'0'),
                    b'a'..=b'f' => Ok(c - b'a' + 10),
                    b'A'..=b'F' => Ok(c - b'A' + 10),
                    _ => Err(Error::InvalidHexString {
                        character: *c as char,
                        hex_string: hex_data.clone(),
                    }),
                })
                .try_fold(0u8, |res, val| Ok(res << 4 | val?))
        })
        .collect()
}

pub fn parse_indirect_reference(object: Pair<Rule>) -> Result<(usize, usize)> {
    let mut reference = find_child_object(object, Rule::indirect_reference)?.into_inner();

    let id = reference.next().unwrap().as_str().parse::<usize>()?;
    let generation = reference.next().unwrap().as_str().parse::<usize>()?;

    Ok((id, generation))
}

pub fn parse_array<T, F>(object: Pair<Rule>, converter: F) -> Result<Vec<T>>
where
    F: Fn(Pair<Rule>) -> Result<T>,
{
    let array = find_child_object(object, Rule::array)?;

    let mut arr = Vec::with_capacity(3);

    for el in array.into_inner() {
        let el = converter(el)?;

        arr.push(el);
    }

    Ok(arr)
}

pub fn parse_dictionary<T, F>(object: Pair<Rule>, mut handler: F) -> Result<T>
where
    T: Default,
    F: FnMut(&mut T, &str, Pair<'_, Rule>) -> Result<()>,
{
    let dictionary = find_child_object(object, Rule::dictionary)?;
    let mut state = T::default();

    for pair in dictionary.into_inner() {
        let mut pair = pair.into_inner();
        let key = pair.next().unwrap().into_inner().next().unwrap().as_str();
        let object = pair.next().unwrap();

        handler(&mut state, key, object)?;
    }

    Ok(state)
}

pub fn parse_stream<T, FnD, FnC>(
    object: Pair<Rule>,
    chunks: &[Vec<u8>],
    expected_type: &str,
    mut dictinary_handler: FnD,
    content_handler: FnC,
) -> Result<T>
where
    T: Default,
    FnD: FnMut(&mut T, &str, Pair<Rule>) -> Result<()>,
    FnC: FnOnce(&T, &[u8]) -> Result<()>,
{
    let mut stream = find_child_object(object, Rule::stream)?.into_inner();
    let dictionary_token = stream.next().unwrap();
    let content_token = stream.next().unwrap();

    let mut content_length: Option<usize> = None;
    let mut filter = StreamFilterType::default();

    let state = parse_dictionary(dictionary_token, |data, key, object| match key {
        "Type" => {
            let stream_type = parse_name(object)?;

            if stream_type != expected_type {
                Err(Error::InvalidStreamType(stream_type.to_string()))
            } else {
                Ok(())
            }
        }
        "Length" => {
            content_length = Some(parse_numeric::<usize>(object)?);

            Ok(())
        }
        "Filter" => {
            let name = parse_name(object)?;
            match name {
                "FlateDecode" => {
                    filter = StreamFilterType::FlateDecode;
                }
                _ => return Err(Error::UnhandledFilterType(name.to_string())),
            }
            Ok(())
        }
        _ => dictinary_handler(data, key, object),
    })?;

    let chunk_id = content_token
        .into_inner()
        .next()
        .unwrap()
        .into_inner()
        .next()
        .unwrap()
        .as_str()
        .parse::<usize>()?;

    let chunk: &Vec<u8> = chunks[chunk_id].as_ref();
    match filter {
        StreamFilterType::None => content_handler(&state, chunk),
        StreamFilterType::FlateDecode => {
            let mut decoder = ZlibDecoder::new(chunk.as_slice());
            let mut data =
                Vec::<u8>::with_capacity(content_length.ok_or(Error::InvalidStreamLength)?);

            decoder.read_to_end(&mut data)?;

            content_handler(&state, data.as_ref())
        }
    }?;

    Ok(state)
}

fn find_child_object(object: Pair<Rule>, rule: Rule) -> Result<Pair<Rule>> {
    let mut token = object.clone();
    while token.as_rule() != rule {
        token = token
            .into_inner()
            .next()
            .ok_or(Error::InvalidTokenPassed(object.to_string(), rule))?;
    }

    Ok(token)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io::Write;

    use super::*;

    use crate::parser::grammar_parser::PDFParser;
    use pest::Parser;

    #[test]
    fn test_find_child_object() {
        struct TestCase {
            input: &'static str,
            target_rule: Rule,
            expected_rule: Option<Rule>,
            successful: bool,
            description: &'static str,
        }

        let cases = vec![
            TestCase {
                input: "123",
                target_rule: Rule::numeric,
                expected_rule: Some(Rule::numeric),
                successful: true,
                description: "Direct rule match",
            },
            TestCase {
                input: "/Example",
                target_rule: Rule::name_data,
                expected_rule: Some(Rule::name_data),
                successful: true,
                description: "One level deep in name",
            },
            TestCase {
                input: "<< /Key 42 >>",
                target_rule: Rule::name,
                expected_rule: Some(Rule::name),
                successful: true,
                description: "Multiple levels in dictionary",
            },
            TestCase {
                input: "true",
                target_rule: Rule::numeric,
                expected_rule: None,
                successful: false,
                description: "Target rule not present",
            },
            TestCase {
                input: "[ 123 /Name ]",
                target_rule: Rule::name,
                expected_rule: None,
                successful: false,
                description: "Target not in first-child path",
            },
            TestCase {
                input: "1 0 R",
                target_rule: Rule::indirect_reference,
                expected_rule: Some(Rule::indirect_reference),
                successful: true,
                description: "Indirect reference structure",
            },
        ];

        for case in cases {
            let pair = PDFParser::parse(Rule::object, case.input)
                .unwrap_or_else(|_| panic!("Parser failed for case: {}", case.description))
                .next()
                .unwrap();

            let result = find_child_object(pair, case.target_rule);

            if !case.successful {
                assert!(
                    result.is_err(),
                    "Case '{}' should error but didn't",
                    case.description
                );
            } else {
                assert!(
                    result.is_ok(),
                    "Case '{}' failed unexpectedly: {:?}",
                    case.description,
                    result
                );
                assert_eq!(
                    result.unwrap().as_rule(),
                    case.expected_rule.unwrap(),
                    "Rule mismatch in case '{}'",
                    case.description
                );
            }
        }
    }

    #[test]
    fn test_parse_numeric() {
        struct TestCase<T> {
            input: &'static str,
            expected: Option<T>,
            successful: bool,
            description: &'static str,
        }

        let cases = vec![
            TestCase {
                input: "123",
                expected: Some(123),
                successful: true,
                description: "Simple integer",
            },
            TestCase {
                input: "-456",
                expected: Some(-456),
                successful: true,
                description: "Negative integer",
            },
            TestCase {
                input: "<<>>",
                expected: None,
                successful: false,
                description: "Non-numeric input",
            },
            TestCase {
                input: "true",
                expected: None,
                successful: false,
                description: "Boolean instead of numeric",
            },
        ];

        for case in cases {
            let pair = PDFParser::parse(Rule::object, case.input)
                .unwrap()
                .next()
                .unwrap();

            let result = parse_numeric::<i32>(pair);

            if case.successful {
                assert!(
                    result.is_ok(),
                    "Result is Error but it shouldn't. Error: {}",
                    result.unwrap_err()
                );

                let result = result.unwrap();
                let expected = case.expected.unwrap();

                assert_eq!(
                    result, expected,
                    "Case failed: {}. Got = {}, expected = {}",
                    case.description, result, expected,
                );
            } else {
                assert!(
                    result.is_err(),
                    "Result is not Error but it should be. Returned value: {}",
                    result.unwrap()
                );
            }
        }
    }

    #[test]
    fn test_parse_name() {
        struct TestCase {
            input: &'static str,
            expected: Option<&'static str>,
            successful: bool,
            description: &'static str,
        }

        let cases = vec![
            TestCase {
                input: "/NormalName",
                expected: Some("NormalName"),
                successful: true,
                description: "Basic name parsing",
            },
            TestCase {
                input: "/",
                expected: Some(""),
                successful: true,
                description: "Empty name",
            },
            TestCase {
                input: "/A#B_123",
                expected: Some("A#B_123"),
                successful: true,
                description: "Special characters in name",
            },
            TestCase {
                input: "123",
                expected: None,
                successful: false,
                description: "Numeric instead of name",
            },
            TestCase {
                input: "<< /Key 42 >>",
                expected: Some("Key"),
                successful: true,
                description: "Dictionary instead of name",
            },
        ];

        for case in cases {
            let pair = PDFParser::parse(Rule::object, case.input)
                .unwrap()
                .next()
                .unwrap();

            let result = parse_name(pair);

            if case.successful {
                assert!(
                    result.is_ok(),
                    "Result is Error but it shouldn't. Result: {:#?}",
                    result
                );

                let result = result.unwrap();
                let expected = case.expected.unwrap();

                assert_eq!(
                    result, expected,
                    "Case failed: {}. Got = {}, expected = {}",
                    case.description, result, expected,
                );
            } else {
                assert!(
                    result.is_err(),
                    "Result is not Error but it should be. Returned value: {:#?}",
                    result
                );
            }
        }
    }

    #[test]
    fn test_parse_hex_string() {
        struct TestCase {
            input: &'static str,
            expected: Option<Vec<u8>>,
            successful: bool,
            description: &'static str,
        }

        let cases = vec![
            TestCase {
                input: "<4142>",
                expected: Some(vec![0x41, 0x42]),
                successful: true,
                description: "Valid even-length hex string",
            },
            TestCase {
                input: "<414>",
                expected: Some(vec![0x41, 0x40]),
                successful: true,
                description: "Odd-length hex string padded with zero",
            },
            TestCase {
                input: "<>",
                expected: Some(vec![]),
                successful: true,
                description: "Empty hex string",
            },
        ];

        for case in cases {
            let pair = PDFParser::parse(Rule::object, case.input)
                .unwrap_or_else(|_| panic!("Parser failed for case: {}", case.description))
                .next()
                .unwrap();

            let result = parse_hex_string(pair);
            if case.successful {
                assert!(
                    result.is_ok(),
                    "Case '{}' failed: {:?}",
                    case.description,
                    result
                );
                let parsed = result.unwrap();
                assert_eq!(
                    parsed,
                    case.expected.unwrap(),
                    "Case '{}' mismatch",
                    case.description
                );
            } else {
                assert!(
                    result.is_err(),
                    "Case '{}' should fail but didn't",
                    case.description
                );
            }
        }
    }

    #[test]
    fn test_parse_indirect_reference() {
        struct TestCase {
            input: &'static str,
            expected: Option<(usize, usize)>,
            successful: bool,
            description: &'static str,
        }

        let cases = vec![
            TestCase {
                input: "123 456 R",
                expected: Some((123, 456)),
                successful: true,
                description: "Valid indirect reference",
            },
            TestCase {
                input: "1 0 R",
                expected: Some((1, 0)),
                successful: true,
                description: "Minimal valid indirect reference",
            },
            TestCase {
                input: "123",
                expected: None,
                successful: false,
                description: "Numeric instead of indirect reference",
            },
            TestCase {
                input: "/Name",
                expected: None,
                successful: false,
                description: "Name instead of indirect reference",
            },
        ];

        for case in cases {
            let pair = PDFParser::parse(Rule::object, case.input)
                .unwrap_or_else(|_| panic!("Parser failed for case: {}", case.description))
                .next()
                .unwrap();

            let result = parse_indirect_reference(pair);

            if case.successful {
                assert!(
                    result.is_ok(),
                    "Case '{}' failed: {:?}",
                    case.description,
                    result
                );
                let parsed = result.unwrap();
                assert_eq!(
                    parsed,
                    case.expected.unwrap(),
                    "Case '{}' mismatch",
                    case.description
                );
            } else {
                assert!(
                    result.is_err(),
                    "Case '{}' should fail but didn't",
                    case.description
                );
            }
        }
    }

    #[test]
    fn test_parse_array() {
        struct TestCase<T> {
            input: &'static str,
            expected: Option<Vec<T>>,
            successful: bool,
            description: &'static str,
        }

        let cases = vec![
            TestCase {
                input: "[ 1 2 3 ]",
                expected: Some(vec![1, 2, 3]),
                successful: true,
                description: "Array of integers",
            },
            TestCase {
                input: "[ 1 /Two 3 ]",
                expected: None,
                successful: false,
                description: "Array with non-numeric element",
            },
            TestCase {
                input: "[]",
                expected: Some(vec![]),
                successful: true,
                description: "Empty array",
            },
        ];

        for case in cases {
            let pair = PDFParser::parse(Rule::object, case.input)
                .unwrap_or_else(|_| panic!("Parser failed for case: {}", case.description))
                .next()
                .unwrap();

            let result = parse_array(pair, parse_numeric::<i32>);

            if case.successful {
                assert!(
                    result.is_ok(),
                    "Case '{}' failed: {:?}",
                    case.description,
                    result
                );
                let parsed = result.unwrap();
                assert_eq!(
                    parsed,
                    case.expected.unwrap(),
                    "Case '{}' mismatch",
                    case.description
                );
            } else {
                assert!(
                    result.is_err(),
                    "Case '{}' should fail but didn't",
                    case.description
                );
            }
        }
    }

    #[test]
    fn test_parse_dictionary() {
        struct TestCase {
            input: &'static str,
            expected: Vec<(&'static str, &'static str)>,
            successful: bool,
            description: &'static str,
        }

        let cases = vec![
            TestCase {
                input: "<< /Key1 42 /Key2 /Value >>",
                expected: vec![("Key1", "42"), ("Key2", "/Value")],
                successful: true,
                description: "Valid dictionary with two entries",
            },
            TestCase {
                input: "<< /Key1 42 /Key2 /Value /Key3 <<>> >>",
                expected: vec![("Key1", "42"), ("Key2", "/Value"), ("Key3", "<<>>")],
                successful: true,
                description: "Valid dictionary with two entries",
            },
            TestCase {
                input: "<<>>",
                expected: vec![],
                successful: true,
                description: "Empty dictionary",
            },
            TestCase {
                input: "<< /Key1 42 /Key2 >>",
                expected: vec![],
                successful: false,
                description: "Malformed dictionary (missing value)",
            },
            TestCase {
                input: "123",
                expected: vec![],
                successful: false,
                description: "Non-dictionary input",
            },
        ];

        for case in cases {
            let parse_result = PDFParser::parse(Rule::object, case.input);
            let pair = match parse_result {
                Ok(mut pairs) => pairs.next().unwrap(),
                Err(_) => {
                    assert!(
                        !case.successful,
                        "Parser failed for case: {}",
                        case.description
                    );
                    continue;
                }
            };

            let result = parse_dictionary(pair, |data: &mut Vec<(String, String)>, key, value| {
                data.push((key.to_string(), value.as_str().to_string()));
                Ok(())
            });

            if case.successful {
                assert!(
                    result.is_ok(),
                    "Case '{}' should succeed but failed: {:?}",
                    case.description,
                    result
                );
                let expected: Vec<(String, String)> = case
                    .expected
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                assert_eq!(
                    result.unwrap(),
                    expected,
                    "Case '{}' key-value mismatch",
                    case.description
                );
            } else {
                assert!(
                    result.is_err(),
                    "Case '{}' should fail but succeeded",
                    case.description
                );
            }
        }
    }

    #[test]
    fn test_parse_stream() {
        struct TestCase {
            input: &'static str,
            chunks: Vec<Vec<u8>>,
            expected_type: &'static str,
            expected_content: Option<Vec<u8>>,
            expected_dict_entries: Vec<(&'static str, &'static str)>,
            successful: bool,
            description: &'static str,
        }

        let cases = vec![
            TestCase {
                input: "<< /Type /XObject /Length 4 >> stream\r\n{ID0}\nendstream",
                chunks: vec![b"test".to_vec()],
                expected_type: "XObject",
                expected_content: Some(b"test".to_vec()),
                expected_dict_entries: vec![],
                successful: true,
                description: "Valid stream with no filter",
            },
            TestCase {
                input: "<< /Type /XObject /Length 4 /Field <123456>>> stream\r\n{ID0}\nendstream",
                chunks: vec![b"test".to_vec()],
                expected_type: "XObject",
                expected_content: Some(b"test".to_vec()),
                expected_dict_entries: vec![("Field", "<123456>")],
                successful: true,
                description: "Valid stream with no filter and custom field",
            },
            TestCase {
                input: "<< /Type /XObject /Filter /FlateDecode /Length 5 >> stream\r\n{ID0}\nendstream",
                chunks: vec![{
                    let mut encoder =
                        flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                    encoder.write_all(b"hello").unwrap();
                    encoder.finish().unwrap()
                }],
                expected_type: "XObject",
                expected_content: Some(b"hello".to_vec()),
                expected_dict_entries: vec![],
                successful: true,
                description: "Valid stream with FlateDecode filter",
            },
            TestCase {
                input: "<< /Type /Xref /Filter /FlateDecode /Length 348 >> stream\r\n{ID0}\nendstream",
                chunks: vec![
                    [
                        120, 218, 45, 210, 181, 78, 4, 81, 20, 128, 225, 115, 89, 96, 97, 129, 93,
                        88, 24, 220, 221, 221, 221, 221, 221, 221, 165, 226, 5, 40, 169, 72, 40,
                        104, 160, 65, 42, 42, 18, 194, 66, 69, 120, 5, 10, 10, 66, 79, 195, 3, 208,
                        194, 220, 63, 183, 249, 114, 243, 159, 153, 201, 77, 206, 136, 136, 252,
                        253, 249, 137, 132, 200, 22, 110, 226, 4, 70, 227, 62, 238, 226, 17, 30,
                        40, 17, 143, 8, 103, 81, 209, 199, 159, 230, 60, 133, 74, 137, 99, 221, 20,
                        63, 156, 81, 226, 244, 153, 226, 192, 57, 244, 199, 0, 12, 68, 39, 6, 97,
                        48, 186, 48, 4, 67, 49, 12, 221, 74, 92, 223, 230, 155, 30, 156, 87, 226,
                        126, 183, 239, 35, 222, 27, 237, 197, 173, 153, 134, 227, 162, 146, 171,
                        83, 83, 34, 112, 73, 201, 93, 171, 41, 94, 92, 81, 114, 95, 161, 223, 125,
                        60, 179, 117, 60, 140, 154, 105, 36, 174, 42, 135, 239, 92, 247, 183, 31,
                        91, 231, 229, 181, 246, 246, 206, 214, 245, 57, 104, 158, 140, 194, 53,
                        229, 250, 242, 218, 221, 202, 248, 208, 102, 62, 153, 169, 133, 213, 152,
                        137, 89, 152, 141, 57, 152, 139, 121, 152, 143, 49, 88, 128, 93, 24, 139,
                        165, 88, 134, 113, 88, 142, 21, 24, 143, 53, 88, 133, 181, 152, 128, 137,
                        88, 135, 245, 216, 128, 141, 216, 132, 205, 216, 130, 237, 152, 132, 29,
                        216, 137, 201, 88, 140, 37, 152, 130, 149, 216, 138, 169, 216, 134, 133,
                        88, 132, 105, 152, 142, 25, 216, 141, 227, 216, 135, 61, 216, 139, 67, 56,
                        136, 253, 56, 128, 163, 56, 140, 35, 56, 134, 27, 56, 141, 147, 184, 128,
                        179, 184, 140, 235, 202, 202, 22, 189, 163, 92, 101, 118, 180, 135, 219,
                        202, 202, 251, 213, 61, 191, 215, 244, 67, 220, 81, 214, 243, 171, 238, 47,
                        30, 253, 231, 159, 216, 219, 255, 7, 239, 213, 57, 247,
                    ]
                    .to_vec(),
                ],
                expected_type: "Xref",
                expected_content: Some(vec![
                    0, 0, 0, 0, 255, 255, 2, 0, 0, 11, 0, 101, 2, 0, 0, 11, 0, 100, 2, 0, 0, 11, 0,
                    83, 2, 0, 0, 11, 0, 22, 2, 0, 0, 11, 0, 106, 2, 0, 0, 11, 0, 104, 2, 0, 0, 11,
                    0, 109, 2, 0, 0, 11, 0, 107, 1, 0, 0, 15, 0, 0, 2, 0, 0, 11, 0, 0, 1, 22, 115,
                    216, 0, 0, 2, 0, 0, 11, 0, 85, 2, 0, 0, 11, 0, 1, 1, 0, 3, 98, 0, 0, 2, 0, 0,
                    11, 0, 2, 2, 0, 0, 11, 0, 87, 1, 0, 7, 181, 0, 0, 2, 0, 0, 11, 0, 3, 2, 0, 0,
                    11, 0, 89, 2, 0, 0, 11, 0, 4, 2, 0, 0, 11, 0, 5, 2, 0, 0, 11, 0, 6, 2, 0, 0,
                    11, 0, 7, 2, 0, 0, 11, 0, 8, 2, 0, 0, 11, 0, 9, 2, 0, 0, 11, 0, 10, 2, 0, 0,
                    11, 0, 11, 2, 0, 0, 11, 0, 12, 2, 0, 0, 11, 0, 13, 2, 0, 0, 11, 0, 14, 1, 0,
                    10, 229, 0, 0, 2, 0, 0, 11, 0, 15, 2, 0, 0, 11, 0, 90, 1, 0, 14, 208, 0, 0, 1,
                    0, 18, 161, 0, 0, 1, 0, 149, 162, 0, 0, 2, 0, 0, 11, 0, 16, 2, 0, 0, 11, 0, 92,
                    1, 0, 155, 136, 0, 0, 2, 0, 0, 11, 0, 17, 2, 0, 0, 11, 0, 93, 1, 0, 163, 64, 0,
                    0, 2, 0, 0, 11, 0, 18, 2, 0, 0, 11, 0, 95, 1, 0, 169, 51, 0, 0, 1, 0, 178, 142,
                    0, 0, 1, 3, 175, 80, 0, 0, 2, 0, 0, 11, 0, 19, 2, 0, 0, 11, 0, 96, 1, 3, 181,
                    145, 0, 0, 1, 3, 191, 234, 0, 0, 1, 7, 154, 160, 0, 0, 1, 7, 162, 163, 0, 0, 1,
                    10, 216, 76, 0, 0, 2, 0, 0, 11, 0, 20, 2, 0, 0, 11, 0, 97, 1, 10, 218, 18, 0,
                    0, 1, 21, 35, 212, 0, 0, 1, 21, 36, 180, 0, 0, 2, 0, 0, 11, 0, 21, 2, 0, 0, 11,
                    0, 54, 2, 0, 0, 11, 0, 36, 2, 0, 0, 11, 0, 37, 2, 0, 0, 11, 0, 38, 2, 0, 0, 11,
                    0, 39, 2, 0, 0, 11, 0, 40, 2, 0, 0, 11, 0, 41, 2, 0, 0, 11, 0, 42, 2, 0, 0, 11,
                    0, 23, 2, 0, 0, 11, 0, 43, 2, 0, 0, 11, 0, 69, 2, 0, 0, 11, 0, 24, 2, 0, 0, 11,
                    0, 48, 2, 0, 0, 11, 0, 49, 2, 0, 0, 11, 0, 25, 2, 0, 0, 11, 0, 50, 2, 0, 0, 11,
                    0, 51, 2, 0, 0, 11, 0, 26, 2, 0, 0, 11, 0, 55, 2, 0, 0, 11, 0, 53, 2, 0, 0, 11,
                    0, 56, 2, 0, 0, 11, 0, 27, 2, 0, 0, 11, 0, 28, 2, 0, 0, 11, 0, 57, 2, 0, 0, 11,
                    0, 58, 2, 0, 0, 11, 0, 59, 2, 0, 0, 11, 0, 60, 2, 0, 0, 11, 0, 61, 2, 0, 0, 11,
                    0, 62, 2, 0, 0, 11, 0, 63, 2, 0, 0, 11, 0, 66, 2, 0, 0, 11, 0, 29, 2, 0, 0, 11,
                    0, 67, 2, 0, 0, 11, 0, 68, 2, 0, 0, 11, 0, 30, 2, 0, 0, 11, 0, 46, 2, 0, 0, 11,
                    0, 47, 2, 0, 0, 11, 0, 31, 2, 0, 0, 11, 0, 52, 2, 0, 0, 11, 0, 64, 2, 0, 0, 11,
                    0, 32, 2, 0, 0, 11, 0, 65, 2, 0, 0, 11, 0, 44, 2, 0, 0, 11, 0, 45, 2, 0, 0, 11,
                    0, 33, 2, 0, 0, 11, 0, 34, 2, 0, 0, 11, 0, 35, 2, 0, 0, 11, 0, 70, 2, 0, 0, 11,
                    0, 82, 2, 0, 0, 11, 0, 73, 2, 0, 0, 11, 0, 71, 2, 0, 0, 11, 0, 72, 2, 0, 0, 11,
                    0, 77, 2, 0, 0, 11, 0, 76, 2, 0, 0, 11, 0, 74, 2, 0, 0, 11, 0, 75, 2, 0, 0, 11,
                    0, 80, 2, 0, 0, 11, 0, 78, 2, 0, 0, 11, 0, 79, 2, 0, 0, 11, 0, 81, 2, 0, 0, 11,
                    0, 99, 2, 0, 0, 11, 0, 86, 2, 0, 0, 11, 0, 84, 2, 0, 0, 11, 0, 91, 2, 0, 0, 11,
                    0, 88, 2, 0, 0, 11, 0, 94, 2, 0, 0, 11, 0, 98, 1, 21, 38, 0, 0, 0, 1, 21, 40,
                    1, 0, 0, 2, 0, 0, 11, 0, 105, 2, 0, 0, 11, 0, 102, 1, 21, 41, 246, 0, 0, 1, 21,
                    42, 72, 0, 0, 2, 0, 0, 11, 0, 108, 2, 0, 0, 11, 0, 103, 1, 21, 182, 189, 0, 0,
                    1, 21, 183, 15, 0, 0, 1, 22, 128, 163, 0, 0,
                ]),
                expected_dict_entries: vec![],
                successful: true,
                description: "Valid stream with preencoded FlateDecode filter",
            },
            TestCase {
                input: "<< /Type /InvalidType /Length 4 >> stream\r\n{ID0}\nendstream",
                chunks: vec![b"test".to_vec()],
                expected_type: "XObject",
                expected_content: None,
                expected_dict_entries: vec![],
                successful: false,
                description: "Invalid stream type",
            },
            TestCase {
                input: "<< /Type /XObject /Filter /FlateDecode >> stream\n{ID0}\nendstream",
                chunks: vec![b"compressed".to_vec()],
                expected_type: "XObject",
                expected_content: None,
                expected_dict_entries: vec![],
                successful: false,
                description: "Missing Length for FlateDecode",
            },
            TestCase {
                input: "<< /Type /XObject /Length 4 /ColorSpace /DeviceRGB >> stream\r\n{ID0}\nendstream",
                chunks: vec![b"test".to_vec()],
                expected_type: "XObject",
                expected_content: Some(b"test".to_vec()),
                expected_dict_entries: vec![("ColorSpace", "/DeviceRGB")],
                successful: true,
                description: "Handles additional dictionary entries",
            },
        ];

        for case in cases {
            let parse_result = PDFParser::parse(Rule::object, case.input);
            let pair = match parse_result {
                Ok(mut pairs) => pairs.next().unwrap(),
                Err(_) => {
                    assert!(
                        !case.successful,
                        "Parser failed for case: {}",
                        case.description
                    );
                    continue;
                }
            };

            let content_result = RefCell::new(None);
            let result = parse_stream(
                pair,
                &case.chunks,
                case.expected_type,
                |data: &mut Vec<(String, String)>, key, value| {
                    eprintln!("{:#?}", data);

                    data.push((key.to_string(), value.as_str().to_string()));

                    Ok(())
                },
                |_, content| {
                    *content_result.borrow_mut() = Some(content.to_vec());
                    Ok(())
                },
            );

            if case.successful {
                assert!(
                    result.is_ok(),
                    "Case '{}' should succeed but failed: {:?}",
                    case.description,
                    result
                );

                // Verify dictionary entries
                let expected_entries: Vec<(String, String)> = case
                    .expected_dict_entries
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                assert_eq!(
                    result.unwrap(),
                    expected_entries,
                    "Case '{}' dictionary mismatch",
                    case.description
                );

                // Verify content
                assert_eq!(
                    content_result.borrow().as_ref().unwrap(),
                    &case.expected_content.unwrap(),
                    "Case '{}' content mismatch",
                    case.description
                );
            } else {
                assert!(
                    result.is_err(),
                    "Case '{}' should fail but succeeded",
                    case.description
                );
            }
        }
    }
}
