/// Represents numeric values in a PDF document according to PDF 2.0 specification.
///
/// PDF supports two types of numeric values:
/// - Integer values (signed 64-bit integers)
/// - Real values (IEEE double-precision floating-point numbers)
///
/// # PDF Specification
/// According to PDF 2.0 specification (ISO 32000-2:2020), numeric objects can be:
/// - Integer values representing whole numbers
/// - Real values representing fractional numbers with optional sign and exponent
///
/// # Usage
/// Numeric values are used throughout PDF documents for:
/// - Object identifiers and generation numbers
/// - Coordinate positions and dimensions in page descriptions
/// - Color values and transformation matrices
/// - Font metrics and character encoding
///
/// # Examples
/// 42              // Integer
/// -17             // Negative integer
/// 3.14            // Real number
/// 123.456e-7      // Real number with exponent
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Numeric {
    /// An integer value stored as a 64-bit signed integer.
    ///
    /// PDF integers have a range of at least -2^31 to 2^31-1 (ISO 32000 requirement),
    /// though this implementation uses i64 for broader compatibility.
    Integer(i64),
    /// A real (floating-point) value stored as IEEE double-precision (64-bit).
    ///
    /// PDF real numbers support optional exponential notation and
    /// have implementation-defined range and precision limits.
    Real(f64),
}
