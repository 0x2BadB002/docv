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
/// ```
/// (Hello World)              // Literal string
/// (Hello\nWorld)             // Literal string with escape
/// (Test\()                   // Literal string with escaped parenthesis
/// <48656C6C6F20576F726C64>  // Hexadecimal string for "Hello World"
/// <4F60 597D>                // Hexadecimal string with spaces (你好 in UTF-16BE)
/// ```
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

impl PdfString {
    /// Returns the underlying byte representation of the PDF string.
    ///
    /// For literal strings, returns the UTF-8 encoded bytes of the string content.
    /// For hexadecimal strings, returns the decoded binary data directly.
    ///
    /// # Returns
    /// A byte slice containing the raw data of the string.
    ///
    /// # Example
    /// ```
    /// let literal = PdfString::Literal("Hello".to_string());
    /// assert_eq!(literal.as_bytes(), b"Hello");
    ///
    /// let hex = PdfString::Hexadecimal(vec![0x48, 0x65, 0x6C, 0x6C, 0x6F]);
    /// assert_eq!(hex.as_bytes(), &[0x48, 0x65, 0x6C, 0x6C, 0x6F]);
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            PdfString::Literal(data) => data.as_bytes(),
            PdfString::Hexadecimal(data) => data.as_slice(),
        }
    }
}
