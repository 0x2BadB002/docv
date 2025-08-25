use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};
use nom::{
    Finish, IResult, Parser,
    bytes::complete::{tag, take_while_m_n},
    character::complete::{digit1, one_of},
    combinator::opt,
    error::Error,
    sequence::preceded,
};

/// Parses a PDF date string into a DateTime with fixed offset.
///
/// PDF date strings follow the format "D:YYYYMMDDHHmmSSOHH'mm'" where components after
/// the year are optional. The timezone offset is optional and defaults to UTC+0.
///
/// # Example
/// "D:20210421143000+02'00" -> DateTime with offset +2 hours
///
/// # Arguments
/// * `input` - String slice to parse
///
/// # Returns
/// `Result` containing:
/// - Remaining input after parsing
/// - `DateTime<FixedOffset>` on success
pub fn string_date(input: &str) -> Result<(&str, chrono::DateTime<FixedOffset>), Error<&str>> {
    let (input, (date, offset)) = (date, opt(timezone)).parse(input).finish()?;

    let offset = offset.unwrap_or(FixedOffset::east_opt(0).unwrap());

    Ok((input, date.and_local_timezone(offset).unwrap()))
}

/// Parses the date portion of a PDF date string.
///
/// The date format is "D:YYYY[MM[DD[HH[mm[SS]]]]]" where components after the year
/// are optional and default to their minimum values (month=1, day=1, time=00:00:00).
///
/// # Arguments
/// * `input` - String slice to parse
///
/// # Returns
/// `IResult` containing:
/// - Remaining input after parsing
/// - `NaiveDateTime` on success
fn date(input: &str) -> IResult<&str, NaiveDateTime> {
    let (input, _) = tag("D:").parse(input)?;

    let (input, (year_str, month_str, day_str, hour_str, minute_str, second_str)) = (
        take_while_m_n(4, 4, |c: char| c.is_ascii_digit()),
        opt(take_while_m_n(2, 2, |c: char| c.is_ascii_digit())),
        opt(take_while_m_n(2, 2, |c: char| c.is_ascii_digit())),
        opt(take_while_m_n(2, 2, |c: char| c.is_ascii_digit())),
        opt(take_while_m_n(2, 2, |c: char| c.is_ascii_digit())),
        opt(take_while_m_n(2, 2, |c: char| c.is_ascii_digit())),
    )
        .parse(input)?;

    let year = year_str.parse().unwrap();
    let month = month_str.map(|s| s.parse().unwrap()).unwrap_or(1u32);
    let day = day_str.map(|s| s.parse().unwrap()).unwrap_or(1u32);
    let hour = hour_str.map(|s| s.parse().unwrap()).unwrap_or(0u32);
    let minute = minute_str.map(|s| s.parse().unwrap()).unwrap_or(0u32);
    let second = second_str.map(|s| s.parse().unwrap()).unwrap_or(0u32);

    let date = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Verify))
    })?;
    let time = NaiveTime::from_hms_opt(hour, minute, second).ok_or_else(|| {
        nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Verify))
    })?;

    Ok((input, NaiveDateTime::new(date, time)))
}

/// Parses the timezone offset portion of a PDF date string.
///
/// The timezone format is either:
/// - 'Z' for UTC
/// - "+HH'mm" for positive offsets
/// - "-HH'mm" for negative offsets
///
/// # Arguments
/// * `input` - String slice to parse
///
/// # Returns
/// `IResult` containing:
/// - Remaining input after parsing
/// - `FixedOffset` on success
fn timezone(input: &str) -> IResult<&str, FixedOffset> {
    let (input, sign) = one_of("+-Z")(input)?;

    if sign == 'Z' {
        return Ok((input, FixedOffset::east_opt(0).unwrap()));
    }

    let (input, (hour, minute)) = (
        digit1.map_res(|s: &str| s.parse::<i32>()),
        opt(preceded(
            tag("'"),
            digit1.map_res(|s: &str| s.parse::<i32>()),
        )),
    )
        .parse(input)?;
    let minute = minute.unwrap_or(0);
    if minute >= 60 {
        return Err(nom::Err::Failure(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Verify,
        )));
    }

    let offset_seconds = (hour * 3600 + minute * 60) * if sign == '+' { 1 } else { -1 };

    FixedOffset::east_opt(offset_seconds)
        .map(|offset| (input, offset))
        .ok_or_else(|| {
            nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Verify))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, FixedOffset, TimeZone};

    #[test]
    fn test_date_parsers() {
        #[derive(Debug)]
        struct TestCase {
            name: &'static str,
            input: &'static str,
            expected: bool,
            expected_result: Option<DateTime<FixedOffset>>,
            expected_remainder: Option<&'static str>,
        }

        let test_cases = [
            // Valid dates
            TestCase {
                name: "valid full date with timezone",
                input: "D:20210421143000+02'00",
                expected: true,
                expected_result: Some(
                    FixedOffset::east_opt(7200)
                        .unwrap()
                        .with_ymd_and_hms(2021, 4, 21, 14, 30, 0)
                        .unwrap(),
                ),
                expected_remainder: Some(""),
            },
            TestCase {
                name: "valid date with UTC timezone",
                input: "D:20210421143000Z",
                expected: true,
                expected_result: Some(
                    FixedOffset::east_opt(0)
                        .unwrap()
                        .with_ymd_and_hms(2021, 4, 21, 14, 30, 0)
                        .unwrap(),
                ),
                expected_remainder: Some(""),
            },
            TestCase {
                name: "valid date with negative timezone",
                input: "D:20210421143000-05'00",
                expected: true,
                expected_result: Some(
                    FixedOffset::west_opt(18000)
                        .unwrap()
                        .with_ymd_and_hms(2021, 4, 21, 14, 30, 0)
                        .unwrap(),
                ),
                expected_remainder: Some(""),
            },
            TestCase {
                name: "valid date without timezone",
                input: "D:20210421143000",
                expected: true,
                expected_result: Some(
                    FixedOffset::east_opt(0)
                        .unwrap()
                        .with_ymd_and_hms(2021, 4, 21, 14, 30, 0)
                        .unwrap(),
                ),
                expected_remainder: Some(""),
            },
            TestCase {
                name: "valid date with partial components",
                input: "D:202104",
                expected: true,
                expected_result: Some(
                    FixedOffset::east_opt(0)
                        .unwrap()
                        .with_ymd_and_hms(2021, 4, 1, 0, 0, 0)
                        .unwrap(),
                ),
                expected_remainder: Some(""),
            },
            TestCase {
                name: "valid date with remainder",
                input: "D:20210421143000+02'00rest",
                expected: true,
                expected_result: Some(
                    FixedOffset::east_opt(7200)
                        .unwrap()
                        .with_ymd_and_hms(2021, 4, 21, 14, 30, 0)
                        .unwrap(),
                ),
                expected_remainder: Some("rest"),
            },
            TestCase {
                name: "valid timezone format with minutes omitted",
                input: "D:20210421143000+02",
                expected: true,
                expected_result: Some(
                    FixedOffset::east_opt(7200)
                        .unwrap()
                        .with_ymd_and_hms(2021, 4, 21, 14, 30, 0)
                        .unwrap(),
                ),
                expected_remainder: Some(""),
            },
            // Invalid dates
            TestCase {
                name: "invalid missing D prefix",
                input: "20210421143000",
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid date components",
                input: "D:20211301143000", // Invalid month 13
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
            TestCase {
                name: "invalid timezone minutes",
                input: "D:20210421143000+02'60", // Invalid minutes 60
                expected: false,
                expected_result: None,
                expected_remainder: None,
            },
        ];

        for case in &test_cases {
            let result = string_date(case.input);
            assert_eq!(
                result.is_ok(),
                case.expected,
                "Test '{}' failed: expected success: {}. Got datetime = {:?}",
                case.name,
                case.expected,
                result.unwrap()
            );

            if case.expected {
                let (actual_remainder, actual_date) = result.unwrap();
                assert_eq!(
                    actual_date,
                    *case.expected_result.as_ref().unwrap(),
                    "Test '{}' failed: expected date: {:?}, got: {:?}",
                    case.name,
                    case.expected_result,
                    actual_date
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
