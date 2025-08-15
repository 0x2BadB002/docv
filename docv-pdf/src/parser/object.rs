use core::str;

use nom::{IResult, Parser, branch::alt};

use crate::parser::{
    DictionaryRecord, Numeric, PdfString, array,
    boolean::boolean,
    dictionary, indirect_object,
    indirect_object::{IndirectObject, IndirectReference},
    indirect_reference,
    name::name,
    null::null,
    numeric, pdf_string, stream,
};
use crate::{Error, Result};

// TODO: Find way to remove Vec from dictionary and stream
// TODO: Implement some helper methods for converting Object to Rust types. Like `parse`

/// Represents fundamental PDF objects as defined in PDF 2.0 specification.
#[derive(Debug, PartialEq, Clone)]
pub enum Object {
    /// A boolean value (true/false literal)
    Boolean(bool),
    /// Numeric values (integer or real numbers)
    Numeric(Numeric),
    /// String values, can be literal "(string)" or hexadecimal "<ffffaa>"
    String(PdfString),
    /// Names starting with '/' followed by a sequence of characters
    Name(std::string::String),
    /// Null object represented by the 'null' literal
    Null,
    /// Array object, contains 0 or more Objects
    Array(Vec<Object>),
    /// Dictionary object, contains key-value pairs
    Dictionary(Vec<DictionaryRecord>),
    /// Stream object, contains key-value pairs and raw byte data
    Stream(Vec<DictionaryRecord>, Vec<u8>),
    IndirectDefinition(IndirectObject),
    IndirectReference(IndirectReference),
}

impl Object {
    pub fn as_integer(&self) -> Result<i64> {
        match self {
            Object::Numeric(Numeric::Integer(data)) => Ok(*data),
            _ => Err(Error::InvalidObjectType {
                expected: "Integer".to_string(),
                got: self.clone(),
            }),
        }
    }
    pub fn as_float(&self) -> Result<f64> {
        match self {
            Object::Numeric(Numeric::Real(data)) => Ok(*data),
            _ => Err(Error::InvalidObjectType {
                expected: "Real".to_string(),
                got: self.clone(),
            }),
        }
    }
    pub fn as_str(&self) -> Result<&str> {
        match self {
            Object::String(PdfString::Literal(data)) => Ok(data.as_str()),
            Object::String(PdfString::Hexadecimal(data)) => Ok(str::from_utf8(data)?),
            _ => Err(Error::InvalidObjectType {
                expected: "String".to_string(),
                got: self.clone(),
            }),
        }
    }
    pub fn as_bytes(&self) -> Result<&[u8]> {
        match self {
            Object::String(data) => Ok(data.as_bytes()),
            _ => Err(Error::InvalidObjectType {
                expected: "String".to_string(),
                got: self.clone(),
            }),
        }
    }
    pub fn as_array(&self) -> Result<&[Object]> {
        match self {
            Object::Array(data) => Ok(data),
            _ => Err(Error::InvalidObjectType {
                expected: "Array".to_string(),
                got: self.clone(),
            }),
        }
    }
    pub fn as_indirect_ref(&self) -> Result<&IndirectReference> {
        match self {
            Object::IndirectReference(id) => Ok(id),
            _ => Err(Error::InvalidObjectType {
                expected: "Array".to_string(),
                got: self.clone(),
            }),
        }
    }
}

/// Parses a PDF object from the input.
///
/// Attempts to parse any of the fundamental PDF object types:
/// - Boolean (`true`/`false`)
/// - Numeric (integer or real)
/// - String (literal or hexadecimal)
/// - Name (e.g., `/Foo`)
/// - Null (`null`)
/// - Array (e.g., `[1 2 /Three]`)
///
/// # Arguments
/// * `input` - Byte slice to parse
///
/// # Returns
/// `IResult` containing remaining input and parsed [`Object`] on success
pub fn object(input: &[u8]) -> IResult<&[u8], Object> {
    alt((
        stream.map(|(record, data)| Object::Stream(record.collect(), data.to_vec())),
        array.map(Object::Array),
        dictionary.map(|res| Object::Dictionary(res.collect())),
        indirect_object.map(Object::IndirectDefinition),
        indirect_reference.map(Object::IndirectReference),
        numeric.map(Object::Numeric),
        pdf_string.map(Object::String),
        name.map(Object::Name),
        boolean.map(Object::Boolean),
        null.map(|_| Object::Null),
    ))
    .parse(input)
}
