use core::str;

use snafu::{OptionExt, Snafu};

use crate::types::{
    Array, Dictionary, IndirectObject, IndirectReference, Numeric, PdfString, Stream,
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
/// true                        // Boolean
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
    ///
    /// # Example
    /// ```
    /// let null_obj = Object::Null;
    /// assert!(null_obj.is_null());
    ///
    /// let num_obj = Object::Numeric(Numeric::Integer(42));
    /// assert!(!num_obj.is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        matches!(self, Object::Null)
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
    ///
    /// # Example
    /// ```
    /// let int_obj = Object::Numeric(Numeric::Integer(42));
    /// let value: i32 = int_obj.as_integer().unwrap();
    /// assert_eq!(value, 42);
    /// ```
    pub fn as_integer<T>(&self) -> Result<T>
    where
        T: TryFrom<i64>,
    {
        match self {
            Object::Numeric(Numeric::Integer(data)) => Ok(TryInto::try_into(*data)
                .ok()
                .with_context(|| error::TypeConvertionSnafu {
                    object: self.clone(),
                })?),
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Integer".to_string(),
                got: self.clone(),
            }
            .into()),
        }
    }

    /// Attempts to convert the object to a floating-point number of type `T`.
    ///
    /// Only succeeds if the object is a `Numeric::Real` and the value
    /// can be converted to the target type `T`.
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
    ///
    /// # Example
    /// ```
    /// let real_obj = Object::Numeric(Numeric::Real(3.14));
    /// let value: f32 = real_obj.as_float().unwrap();
    /// assert_eq!(value, 3.14);
    /// ```
    pub fn as_float<T>(&self) -> Result<T>
    where
        T: TryFrom<f64>,
    {
        match self {
            Object::Numeric(Numeric::Real(data)) => Ok(TryInto::try_into(*data)
                .ok()
                .with_context(|| error::TypeConvertionSnafu {
                    object: self.clone(),
                })?),
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Real".to_string(),
                got: self.clone(),
            }
            .into()),
        }
    }

    pub fn as_bool(&self) -> Result<bool> {
        match self {
            Object::Boolean(data) => Ok(*data),
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Boolean".to_string(),
                got: self.clone(),
            }
            .into()),
        }
    }

    pub fn as_name(&self) -> Result<&str> {
        match self {
            Object::Name(name) => Ok(name),
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Name".to_string(),
                got: self.clone(),
            }
            .into()),
        }
    }

    /// Attempts to convert the object to an array slice.
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
    ///
    /// # Example
    /// ```
    /// let array_obj = Object::Array(vec![Object::Null]);
    /// let value = array_obj.as_array().unwrap();
    /// assert_eq!(value, &[Object::Null]);
    /// ```
    pub fn as_array(&self) -> Result<&Array> {
        match self {
            Object::Array(data) => Ok(data),
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Array".to_string(),
                got: self.clone(),
            }
            .into()),
        }
    }

    pub fn as_string(&self) -> Result<&PdfString> {
        match self {
            Object::String(data) => Ok(data),
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "String".to_string(),
                got: self.clone(),
            }
            .into()),
        }
    }

    pub fn as_dictionary(&self) -> Result<&Dictionary> {
        match self {
            Object::Dictionary(data) => Ok(data),
            Object::IndirectDefinition(data) => {
                let data = match data.get_object() {
                    Object::Dictionary(data) => Ok(data),
                    _ => Err(error::Error::UnexpectedObjectType {
                        expected: "Dictionary".to_string(),
                        got: self.clone(),
                    }),
                }?;

                Ok(data)
            }
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Dictionary".to_string(),
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
    ///
    /// # Example
    /// ```
    /// let ref_obj = Object::IndirectReference(IndirectReference::new(1, 0));
    /// let value = ref_obj.as_indirect_ref().unwrap();
    /// assert_eq!(value, &IndirectReference::new(1, 0));
    /// ```
    pub fn as_indirect_ref(&self) -> Result<&IndirectReference> {
        match self {
            Object::IndirectReference(id) => Ok(id),
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Indirect reference".to_string(),
                got: self.clone(),
            }
            .into()),
        }
    }

    /// Attempts to convert the object to a stream.
    ///
    /// Only succeeds if the object is an `Object::Stream`.
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
    ///
    /// # Example
    /// ```
    /// let stream_obj = Object::Stream(Stream::new(Dictionary::new(), vec![]));
    /// let value = stream_obj.as_stream().unwrap();
    /// assert_eq!(value.dictionary(), &Dictionary::new());
    /// ```
    pub fn as_stream(&self) -> Result<&Stream> {
        match self {
            Object::Stream(stream) => Ok(stream),
            Object::IndirectDefinition(data) => {
                let data = match data.get_object() {
                    Object::Stream(data) => Ok(data),
                    _ => Err(error::Error::UnexpectedObjectType {
                        expected: "Stream".to_string(),
                        got: self.clone(),
                    }),
                }?;

                Ok(data)
            }
            _ => Err(error::Error::UnexpectedObjectType {
                expected: "Stream".to_string(),
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
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Unexpected object type. Expected = {expected}. Got = {got:?}"))]
        UnexpectedObjectType { expected: String, got: Object },

        #[snafu(display("Can't convert into Rust type. Object = {object:?}"))]
        TypeConvertion { object: Object },
    }
}
