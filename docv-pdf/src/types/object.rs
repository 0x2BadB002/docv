use core::str;
use std::borrow::Cow;

use snafu::{OptionExt, Snafu};

use crate::{
    objects::Objects,
    types::{
        Array, Dictionary, IndirectObject, IndirectReference, Numeric, PdfString, Stream,
        array::ArrayBuilder,
    },
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

/// Represents all fundamental object types defined in the PDF 2.0 specification.
///
/// PDF documents are built from a hierarchy of objects that can be:
/// - Simple atomic values (boolean, numeric, string, name, null)
/// - Complex structures (array, dictionary, stream)
/// - Indirect references and object definitions
///
/// # PDF Object Types
/// According to PDF 2.0 specification (ISO 32000-2:2020), objects can be:
/// - Boolean: `true` or `false` literals
/// - Numeric: Integer or real numbers
/// - String: Literal strings in parentheses or hexadecimal strings in angle brackets
/// - Name: Unique identifier starting with '/' followed by characters
/// - Null: Represented by the `null` keyword
/// - Array: Ordered collection of objects
/// - Dictionary: Collection of key-value pairs with name keys
/// - Stream: Dictionary followed by binary data
/// - Indirect: Object references and definitions for cross-referencing
///
/// # Examples
/// true                       // Boolean
/// 42                         // Numeric (Integer)
/// 3.14                       // Numeric (Real)
/// (Hello World)              // String (Literal)
/// <48656C6C6F>               // String (Hexadecimal)
/// /Type                      // Name
/// null                       // Null
/// [1 2 3]                    // Array
/// << /Key /Value >>          // Dictionary
/// 1 0 obj << /Length 10 >> stream ... endstream // Stream
/// 1 0 R                      // Indirect Reference
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
    Array(Array),
    /// Dictionary object, contains key-value pairs
    Dictionary(Dictionary),
    /// Stream object, contains key-value pairs and raw byte data
    Stream(Stream),
    /// Indirect object definition
    IndirectDefinition(IndirectObject),
    /// Indirect reference to an object, used to reference objects defined elsewhere in the PDF
    IndirectReference(IndirectReference),
}

impl Object {
    /// Checks if the object is a null object.
    ///
    /// # Returns
    /// `true` if the object is `Object::Null`, `false` otherwise.
    pub fn is_null(&self) -> bool {
        matches!(self, Object::Null)
    }

    pub fn direct<'a>(&'a self, objects: &mut Objects) -> Cow<'a, Object> {
        match self {
            Object::IndirectReference(obj_ref) => objects
                .get_object(obj_ref)
                .map_or(Cow::Borrowed(self), Cow::Owned),
            _ => Cow::Borrowed(self),
        }
    }

    /// Attempts to convert the object to an integer of type `T`.
    ///
    /// Only succeeds if the object is a `Numeric::Integer` and the value
    /// can be converted to the target type `T`.
    ///
    /// # Arguments
    /// * `self` - Reference to the object
    ///
    /// # Returns
    /// - `Ok(T)` if conversion is successful
    /// - `Err(Error)` if the object is not an integer or conversion fails
    ///
    /// # Errors
    /// Returns `Error::UnexpectedObjectType` if the object is not a numeric integer.
    /// Returns `Error::TypeConvertion` if the integer value cannot be converted to type `T`.
    pub fn as_integer<T>(&self) -> Result<T>
    where
        T: TryFrom<i64>,
    {
        match self {
            Object::Numeric(Numeric::Integer(data)) => Ok(TryInto::try_into(*data)
                .ok()
                .with_context(|| error::TypeConvertion {
                    object: self.clone(),
                })?),
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Integer",
                got: self.clone(),
            }
            .into()),
        }
    }

    /// Attempts to convert the object to a floating-point number of type `f64`.
    ///
    /// Only succeeds if the object is a `Numeric`.
    ///
    /// # Arguments
    /// * `self` - Reference to the object
    ///
    /// # Returns
    /// - `Ok(T)` if conversion is successful
    /// - `Err(Error)` if the object is not a real number or conversion fails
    ///
    /// # Errors
    /// Returns `Error::UnexpectedObjectType` if the object is not a numeric real.
    /// Returns `Error::TypeConvertion` if the real value cannot be converted to type `T`.
    pub fn as_float(&self) -> Result<f64> {
        match self {
            Object::Numeric(Numeric::Integer(data)) => Ok(*data as f64),
            Object::Numeric(Numeric::Real(data)) => Ok(*data),
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Real",
                got: self.clone(),
            }
            .into()),
        }
    }

    /// Attempts to convert the object to a boolean value.
    ///
    /// Only succeeds if the object is an `Object::Boolean`.
    ///
    /// # Arguments
    /// * `self` - Reference to the object
    ///
    /// # Returns
    /// - `Ok(bool)` containing the boolean value if successful
    /// - `Err(Error)` if the object is not a boolean
    ///
    /// # Errors
    /// Returns `Error::UnexpectedObjectType` if the object is not a boolean.
    pub fn as_bool(&self) -> Result<bool> {
        match self {
            Object::Boolean(data) => Ok(*data),
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Boolean",
                got: self.clone(),
            }
            .into()),
        }
    }

    /// Attempts to convert the object to a name string slice.
    ///
    /// Only succeeds if the object is an `Object::Name`.
    ///
    /// # Arguments
    /// * `self` - Reference to the object
    ///
    /// # Returns
    /// - `Ok(&str)` containing the name string if successful
    /// - `Err(Error)` if the object is not a name
    ///
    /// # Errors
    /// Returns `Error::UnexpectedObjectType` if the object is not a name.
    pub fn as_name(&self) -> Result<&str> {
        match self {
            Object::Name(name) => Ok(name),
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Name",
                got: self.clone(),
            }
            .into()),
        }
    }

    /// Attempts to convert the object to an array slice converting
    /// array contents into specific type.
    ///
    /// If you do not want to have indirect references in resulting
    /// array, use `.with_objects(...)` before using `.of(...)` method.
    ///
    /// If you want array of `Object` without converting then use `.generic()`.
    ///
    /// Only succeeds if the object is an `Object::Array`.
    ///
    /// # Arguments
    /// * `self` - Reference to the object
    ///
    /// # Returns
    /// - `Ok(&[Object])` if the object is an array
    /// - `Err(Error)` if the object is not an array
    ///
    /// # Errors
    /// Returns `Error::UnexpectedObjectType` if the object is not an array.
    pub fn as_array<'a>(&'a self) -> ArrayBuilder<'a> {
        ArrayBuilder::new(self)
    }

    /// Attempts to convert the object to a PDF string.
    ///
    /// Only succeeds if the object is an `Object::String`.
    ///
    /// # Arguments
    /// * `self` - Reference to the object
    ///
    /// # Returns
    /// - `Ok(&PdfString)` containing the PDF string if successful
    /// - `Err(Error)` if the object is not a string
    ///
    /// # Errors
    /// Returns `Error::UnexpectedObjectType` if the object is not a string.
    pub fn as_string(&self) -> Result<&PdfString> {
        match self {
            Object::String(data) => Ok(data),
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "String",
                got: self.clone(),
            }
            .into()),
        }
    }

    /// Attempts to convert the object to a dictionary.
    ///
    /// Succeeds if the object is either:
    /// - `Object::Dictionary`
    /// - `Object::IndirectDefinition` containing a dictionary
    ///
    /// # Arguments
    /// * `self` - Reference to the object
    ///
    /// # Returns
    /// - `Ok(&Dictionary)` containing the dictionary if successful
    /// - `Err(Error)` if the object is not a dictionary
    ///
    /// # Errors
    /// Returns `Error::UnexpectedObjectType` if the object is not a dictionary
    /// or an indirect definition containing a dictionary.
    pub fn as_dictionary(&self) -> Result<&Dictionary> {
        match self {
            Object::Dictionary(data) => Ok(data),
            Object::IndirectDefinition(data) => {
                let data = match &**data {
                    Object::Dictionary(data) => Ok(data),
                    _ => Err(error::Error::UnexpectedObjectType {
                        expected: "Dictionary",
                        got: self.clone(),
                    }),
                }?;

                Ok(data)
            }
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Dictionary",
                got: self.clone(),
            }
            .into()),
        }
    }

    /// Attempts to convert the object to an indirect reference.
    ///
    /// Only succeeds if the object is an `Object::IndirectReference`.
    ///
    /// # Arguments
    /// * `self` - Reference to the object
    ///
    /// # Returns
    /// - `Ok(&IndirectReference)` containing the reference if successful
    /// - `Err(Error)` if the object is not an indirect reference
    ///
    /// # Errors
    /// Returns `Error::UnexpectedObjectType` if the object is not an indirect reference.
    pub fn as_indirect_ref(&self) -> Result<&IndirectReference> {
        match self {
            Object::IndirectReference(id) => Ok(id),
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Indirect reference",
                got: self.clone(),
            }
            .into()),
        }
    }

    /// Attempts to convert the object to a stream.
    ///
    /// Succeeds if the object is either:
    /// - `Object::Stream`
    /// - `Object::IndirectDefinition` containing a stream
    ///
    /// # Arguments
    /// * `self` - Reference to the object
    ///
    /// # Returns
    /// - `Ok(&Stream)` containing the stream if successful
    /// - `Err(Error)` if the object is not a stream
    ///
    /// # Errors
    /// Returns `Error::UnexpectedObjectType` if the object is not a stream.
    pub fn as_stream(&self) -> Result<&Stream> {
        match self {
            Object::Stream(stream) => Ok(stream),
            Object::IndirectDefinition(data) => {
                let data = match &**data {
                    Object::Stream(data) => Ok(data),
                    _ => Err(error::Error::UnexpectedObjectType {
                        expected: "Stream",
                        got: self.clone(),
                    }),
                }?;

                Ok(data)
            }
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Stream",
                got: self.clone(),
            }
            .into()),
        }
    }
}

mod error {
    use core::str;

    use snafu::Snafu;

    use super::Object;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Unexpected object type. Expected = {expected}. Got = {got:?}"))]
        UnexpectedObjectType { expected: &'static str, got: Object },

        #[snafu(display("Can't convert into Rust type. Object = {object:?}"))]
        TypeConvertion { object: Object },
    }
}
