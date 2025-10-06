use chrono::{DateTime, FixedOffset};
use snafu::{OptionExt, ResultExt, Snafu};

use crate::parser::read_date;

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

/// Represents string values in a PDF document according to PDF 2.0 specification.
///
/// PDF supports two types of string objects:
/// - Literal strings: Enclosed in parentheses `(content)` with support for escape sequences
/// - Hexadecimal strings: Enclosed in angle brackets `<hex data>` representing binary data
///
/// # PDF String Types
/// According to PDF 2.0 specification (ISO 32000-2:2020):
///
/// ## Literal Strings
/// - Enclosed in parentheses `(string content)`
/// - Support escape sequences: `\n`, `\r`, `\t`, `\b`, `\f`, `\(`, `\)`, `\\`
/// - Can span multiple lines using line continuation with backslash
///
/// ## Hexadecimal Strings
/// - Enclosed in angle brackets `<48656C6C6F>`
/// - Represent binary data as hexadecimal digits
/// - Each pair of hex digits represents one byte
/// - White space between hex digits is ignored
/// - Odd number of digits: last digit assumed to be 0 (e.g., `<ABC>` becomes `<AB C0>`)
///
/// # Usage
/// PDF strings are used for:
/// - Text content in page descriptions
/// - Dictionary values and metadata
/// - File names and document information
/// - JavaScript code and form field values
///
/// # Examples
/// (Hello World)              // Literal string
/// (Hello\nWorld)             // Literal string with escape
/// (Test\()                   // Literal string with escaped parenthesis
/// <48656C6C6F20576F726C64>  // Hexadecimal string for "Hello World"
/// <4F60 597D>                // Hexadecimal string with spaces (你好 in UTF-16BE)
#[derive(Debug, PartialEq, Clone)]
pub enum PdfString {
    /// A literal string enclosed in parentheses with support for escape sequences.
    ///
    /// PDF literal strings can contain arbitrary characters with certain characters
    /// requiring escape sequences. The content is stored after processing escapes.
    Literal(std::string::String),
    /// A hexadecimal string representing binary data enclosed in angle brackets.
    ///
    /// Hexadecimal strings store raw byte data as pairs of hexadecimal digits.
    /// The content is stored as decoded bytes rather than the original text representation.
    Hexadecimal(Vec<u8>),
}

/// Represents a date and time value in a PDF document.
///
/// PDF dates are represented as strings in the format:
/// `(D:YYYYMMDDHHmmSSOHH'mm')`
///
/// Where:
/// - `YYYY` = year (0000-9999)
/// - `MM` = month (01-12)
/// - `DD` = day (01-31)
/// - `HH` = hour (00-23)
/// - `mm` = minute (00-59)
/// - `SS` = second (00-59)
/// - `O` = UTC relationship (`+`, `-`, or `Z` for UTC)
/// - `HH'` = time zone hour offset (00-23)
/// - `mm'` = time zone minute offset (00-59)
///
/// The time zone offset and seconds are optional. If no time zone is specified,
/// the date is interpreted as local time.
///
/// # Examples
/// (D:20231231093000)        // Local time: Dec 31, 2023, 09:30:00
/// (D:20231231093000Z)       // UTC: Dec 31, 2023, 09:30:00
/// (D:20231231093000-05'00') // EST: Dec 31, 2023, 09:30:00 (UTC-5)
#[derive(Debug, Default, Clone, Copy)]
pub struct Date {
    data: DateTime<FixedOffset>,
}

impl PdfString {
    /// Attempts to convert the PDF string to a UTF-8 string slice.
    ///
    /// For literal strings, returns the content as-is after processing escape sequences.
    /// For hexadecimal strings, attempts to decode the bytes as UTF-8.
    ///
    /// # Returns
    /// - `Ok(&str)` containing the string content if successful
    /// - `Err(Error)` if the hexadecimal data cannot be decoded as UTF-8
    ///
    /// # Errors
    /// Returns `Error::EncodingStr` if the hexadecimal string contains invalid UTF-8 data.
    ///
    /// # Note
    /// Removes Byte Order Mark (BOM) `\u{FEFF}` from the beginning of strings if present.
    pub fn as_str(&self) -> Result<&str> {
        match self {
            PdfString::Literal(data) => {
                let mut data = data.as_str();

                data = data.trim_start_matches("\u{FEFF}");

                Ok(data)
            }
            PdfString::Hexadecimal(data) => {
                let mut data = str::from_utf8(data).with_context(|_| error::EncodingStr {
                    data: data.to_vec(),
                })?;

                data = data.trim_start_matches("\u{FEFF}");

                Ok(data)
            }
        }
    }

    /// Returns the raw byte representation of the PDF string.
    ///
    /// For literal strings, returns the UTF-8 bytes of the string content.
    /// For hexadecimal strings, returns the decoded byte data.
    ///
    /// # Returns
    /// `&[u8]` slice containing the raw byte data
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            PdfString::Literal(data) => data.as_bytes(),
            PdfString::Hexadecimal(data) => data.as_slice(),
        }
    }

    /// Attempts to parse the PDF string as a date value.
    ///
    /// PDF dates follow the format: `(D:YYYYMMDDHHmmSSOHH'mm')`
    ///
    /// # Returns
    /// - `Ok(Date)` containing the parsed date if successful
    /// - `Err(Error)` if the string cannot be parsed as a valid PDF date
    ///
    /// # Errors
    /// Returns `Error::ParseTo` if the string does not match the expected PDF date format
    /// or contains invalid date components.
    pub fn to_date(&self) -> Result<Date> {
        let input = self.as_str()?;
        let (_, date) = read_date(input).ok().with_context(|| error::ParseTo {
            data: input.to_string(),
            target: "date",
        })?;

        Ok(Date { data: date })
    }
}

impl From<String> for PdfString {
    fn from(value: String) -> Self {
        Self::Literal(value)
    }
}

impl<'a> From<&'a str> for PdfString {
    fn from(value: &'a str) -> Self {
        Self::Literal(value.to_string())
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.data.fmt(f)
    }
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Can't encode into UTF-8. Data = {data:?}"))]
        EncodingStr {
            data: Vec<u8>,
            source: std::str::Utf8Error,
        },

        #[snafu(display("Can't parse inner string '{data}' into {target}"))]
        ParseTo { data: String, target: &'static str },
    }
}
