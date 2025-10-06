use core::str;

use nom::{
    Finish, Parser,
    branch::alt,
    character::complete::digit1,
    error::Error,
    multi::{count, many0},
    sequence::terminated,
};

use super::whitespace::{comment, eol, whitespace};

/// Parses the header of an object stream containing object ID and offset pairs.
///
/// Object stream headers consist of `n` pairs of integers representing:
/// - Object ID: The object number within the stream
/// - Offset: Byte offset from the start of the stream's content where the object begins
///
/// The header format allows optional whitespace, comments, and line endings between values.
/// Each pair is parsed as two integers separated by whitespace.
///
/// # Format
/// ```text
/// obj_id_1 offset_1
/// obj_id_2 offset_2
/// ...
/// obj_id_n offset_n
/// ```
///
/// # Example
/// ```text
/// 123 456
/// 789 012
/// 345 678
/// ```
///
/// # Arguments
/// * `input` - Byte slice containing the object stream header data
/// * `n` - Number of object ID and offset pairs to parse
///
/// # Returns
/// `Result` containing:
/// - `Vec<(usize, usize)>` with object ID and offset pairs on success
/// - `Error` if parsing fails, insufficient data, or non-numeric values encountered
///
/// # Notes
/// - The parser is lenient about whitespace and allows comments between values
/// - Returns exactly `n` pairs if successful, even if more data is available
/// - Object IDs and offsets are parsed as `usize` values
pub fn read_object_stream_header(
    input: &[u8],
    n: usize,
) -> Result<Vec<(usize, usize)>, Error<&[u8]>> {
    count(
        (
            terminated(
                digit1.map_res(|s| str::from_utf8(s)),
                many0(alt((whitespace, comment, eol))),
            )
            .map_res(|s| s.parse::<usize>()),
            terminated(
                digit1.map_res(|s| str::from_utf8(s)),
                many0(alt((whitespace, comment, eol))),
            )
            .map_res(|s| s.parse::<usize>()),
        ),
        n,
    )
    .parse(input)
    .finish()
    .map(|(_, res)| res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_stream_header_parser() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static [u8],
            n: usize,
            expected: bool,
            expected_value: Option<Vec<(usize, usize)>>,
        }

        let test_cases = [
            // Valid object stream headers
            TestCase {
                name: "valid single object stream header",
                input: b"123 456",
                n: 1,
                expected: true,
                expected_value: Some(vec![(123, 456)]),
            },
            TestCase {
                name: "valid multiple object stream headers",
                input: b"123 456\n789 012\n345 678",
                n: 3,
                expected: true,
                expected_value: Some(vec![(123, 456), (789, 12), (345, 678)]),
            },
            TestCase {
                name: "valid object stream headers with whitespace",
                input: b"123 456 \n 789 012 \t\n 345 678",
                n: 3,
                expected: true,
                expected_value: Some(vec![(123, 456), (789, 12), (345, 678)]),
            },
            TestCase {
                name: "valid object stream headers with comments",
                input: b"123 456 % comment\n789 012 % another comment\n345 678",
                n: 3,
                expected: true,
                expected_value: Some(vec![(123, 456), (789, 12), (345, 678)]),
            },
            TestCase {
                name: "valid object stream headers mixed separators",
                input: b"123 456\n789 012\t345 678",
                n: 3,
                expected: true,
                expected_value: Some(vec![(123, 456), (789, 12), (345, 678)]),
            },
            TestCase {
                name: "valid object stream headers with extra content",
                input: b"123 456\n789 012\n345 678\n999 000",
                n: 3,
                expected: true,
                expected_value: Some(vec![(123, 456), (789, 12), (345, 678)]),
            },
            // Invalid object stream headers
            TestCase {
                name: "invalid insufficient data",
                input: b"123 456",
                n: 2,
                expected: false,
                expected_value: None,
            },
            TestCase {
                name: "invalid non-numeric first value",
                input: b"abc 456",
                n: 1,
                expected: false,
                expected_value: None,
            },
            TestCase {
                name: "invalid non-numeric second value",
                input: b"123 def",
                n: 1,
                expected: false,
                expected_value: None,
            },
            TestCase {
                name: "invalid missing second value",
                input: b"123",
                n: 1,
                expected: false,
                expected_value: None,
            },
            TestCase {
                name: "invalid empty input",
                input: b"",
                n: 1,
                expected: false,
                expected_value: None,
            },
            TestCase {
                name: "zero count",
                input: b"123 456",
                n: 0,
                expected: true,
                expected_value: Some(vec![]),
            },
        ];

        for case in &test_cases {
            let result = read_object_stream_header(case.input, case.n);
            assert_eq!(
                result.is_ok(),
                case.expected,
                "Test '{}' failed: expected success: {}, got: {:?}",
                case.name,
                case.expected,
                result
            );

            if case.expected {
                let actual_value = result.unwrap();
                assert_eq!(
                    actual_value,
                    *case.expected_value.as_ref().unwrap(),
                    "Test '{}' failed: expected value: {:?}, got: {:?}",
                    case.name,
                    case.expected_value,
                    actual_value
                );
            }
        }
    }

    #[test]
    fn test_object_stream_header_edge_cases() {
        // Test with maximum usize values
        let max_usize = usize::MAX.to_string();
        let input = format!("{} {}", max_usize, max_usize).into_bytes();
        let result = read_object_stream_header(&input, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![(usize::MAX, usize::MAX)]);

        // Test with minimum values
        let input = b"0 0";
        let result = read_object_stream_header(input, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![(0, 0)]);

        // Test with very large count (though in practice object streams are limited)
        let input = b"1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20";
        let result = read_object_stream_header(input, 10);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            vec![
                (1, 2),
                (3, 4),
                (5, 6),
                (7, 8),
                (9, 10),
                (11, 12),
                (13, 14),
                (15, 16),
                (17, 18),
                (19, 20)
            ]
        );
    }
}
