use snafu::{ResultExt, Snafu, ensure};

pub mod rectangle;

use crate::{
    objects::Objects,
    types::{Object, Rectangle},
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

/// A PDF array object that contains an ordered collection of PDF objects.
///
/// Arrays are represented as a sequence of objects enclosed in square brackets.
/// According to the PDF specification, arrays can contain any combination of
/// object types, including other arrays and dictionaries.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Array {
    data: Vec<Object>,
}

/// A builder for constructing and processing PDF arrays.
///
/// `ArrayBuilder` follows the builder pattern to provide a fluent interface
/// for working with PDF arrays. It supports:
/// - Extracting raw arrays with indirect reference resolution
/// - Converting array elements to specific types
/// - Optional object resolution for handling indirect references
pub struct ArrayBuilder<'a> {
    array: &'a Object,
    objects: Option<&'a mut Objects>,
}

impl<'a> ArrayBuilder<'a> {
    /// Creates a new `ArrayBuilder` for the given array object.
    ///
    /// # Arguments
    /// * `array` - A reference to the PDF array object to process
    ///
    /// # Panics
    /// This method does not panic, but subsequent operations will fail if the
    /// provided object is not actually an array.
    pub fn new(array: &'a Object) -> Self {
        Self {
            array,
            objects: None,
        }
    }

    /// Provides an object store for resolving indirect references.
    ///
    /// When processing arrays that contain indirect references (e.g., `1 0 R`),
    /// this method allows the builder to resolve those references to their
    /// actual objects using the provided [`Objects`] store.
    ///
    /// # Arguments
    /// * `objects` - A mutable reference to the object store for resolution
    ///
    /// # Returns
    /// Returns `&mut Self` for method chaining.
    pub fn with_objects(&mut self, objects: &'a mut Objects) -> &mut Self {
        self.objects = Some(objects);
        self
    }

    /// Extracts the array as a generic vector of PDF objects.
    ///
    /// This method returns the array contents as a vector of [`Object`] instances.
    /// If an object store was provided via [`with_objects`], indirect references
    /// will be resolved to their actual objects.
    ///
    /// # Returns
    /// - `Ok(Array)` containing the processed array objects
    /// - `Err(Error)` if the input object is not an array or if object resolution fails
    ///
    /// # Errors
    /// Returns an error if:
    /// - The input object is not an array
    /// - Object resolution fails (when using [`with_objects`])
    ///
    /// [`with_objects`]: ArrayBuilder::with_objects
    pub fn generic(&mut self) -> Result<Array> {
        let array = match self.array {
            Object::Array(arr) => arr,
            _ => {
                return Err(error::Error::UnexpectedObjectType {
                    expected: "Array",
                    got: self.array.clone(),
                }
                .into());
            }
        };

        match self.objects.as_mut() {
            Some(objects) => {
                let mut res = Vec::with_capacity(array.len());
                for object in array.iter() {
                    let object = match object {
                        Object::IndirectReference(obj_ref) => {
                            objects.get_object(obj_ref).context(error::ObjectNotFound {
                                reference: *obj_ref,
                            })?
                        }
                        _ => object.clone(),
                    };

                    res.push(object);
                }

                Ok(res.into())
            }
            None => Ok(array.clone()),
        }
    }

    /// Converts the array elements to a specific type using a conversion function.
    ///
    /// This method processes each element in the array using the provided conversion
    /// function. If an object store was provided, indirect references are resolved
    /// before conversion.
    ///
    /// # Type Parameters
    /// * `F` - The conversion function type
    /// * `O` - The output type of the conversion
    ///
    /// # Arguments
    /// * `init_fn` - A function that converts a PDF object to the desired type
    ///
    /// # Returns
    /// - `Ok(Vec<O>)` containing the converted elements
    /// - `Err(Error)` if conversion fails or the input is not an array
    ///
    /// # Errors
    /// Returns an error if:
    /// - The input object is not an array
    /// - Object resolution fails (when using [`with_objects`])
    /// - The conversion function fails for any element
    ///
    /// [`with_objects`]: ArrayBuilder::with_objects
    pub fn of<F, O>(&mut self, init_fn: F) -> Result<Vec<O>>
    where
        F: Fn(&Object) -> std::result::Result<O, crate::types::object::Error>,
    {
        let array = match self.array {
            Object::Array(arr) => arr,
            _ => {
                return Err(error::Error::UnexpectedObjectType {
                    expected: "Array",
                    got: self.array.clone(),
                }
                .into());
            }
        };

        match self.objects.as_mut() {
            Some(objects) => {
                let mut res = Vec::with_capacity(array.len());
                for object in array.iter() {
                    let object = match object {
                        Object::IndirectReference(obj_ref) => {
                            objects.get_object(obj_ref).context(error::ObjectNotFound {
                                reference: *obj_ref,
                            })?
                        }
                        _ => object.clone(),
                    };
                    let object = init_fn(&object).context(error::FailedArrayConvertion)?;

                    res.push(object);
                }

                Ok(res)
            }
            None => Ok(array
                .iter()
                .map(init_fn)
                .collect::<std::result::Result<Vec<_>, _>>()
                .context(error::FailedArrayConvertion)?),
        }
    }

    /// Converts the array to a PDF rectangle.
    ///
    /// PDF rectangles are represented as arrays of four numbers: `[x1, y1, x2, y2]`.
    /// This method extracts the coordinates and returns a normalized `Rectangle`
    /// where coordinates are ordered as left, bottom, right, top.
    ///
    /// # Returns
    /// - `Ok(Rectangle)` containing the normalized rectangle coordinates
    /// - `Err(Error)` if conversion fails
    ///
    /// # Errors
    /// Returns an error if:
    /// - The input object is not an array
    /// - The array does not contain exactly 4 elements
    /// - Any element cannot be converted to a numeric value
    /// - Object resolution fails (when using [`with_objects`])
    ///
    /// [`with_objects`]: ArrayBuilder::with_objects
    pub fn rectangle(&mut self) -> Result<Rectangle> {
        let coords: Vec<f64> = self.of(|obj| obj.as_float())?;

        ensure!(
            coords.len() == 4,
            error::InvalidRectangleFormat {
                expected: 4usize,
                got: coords.len()
            }
        );

        Ok(Rectangle::new(coords[0], coords[1], coords[2], coords[3]))
    }
}

impl std::ops::Deref for Array {
    type Target = Vec<Object>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl From<Vec<Object>> for Array {
    fn from(value: Vec<Object>) -> Self {
        Self { data: value }
    }
}

impl<const N: usize> From<[Object; N]> for Array {
    fn from(value: [Object; N]) -> Self {
        Self {
            data: value.to_vec(),
        }
    }
}

mod error {
    use super::*;

    use core::str;

    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Unexpected object type. Expected = {expected}. Got = {got:?}"))]
        UnexpectedObjectType { expected: &'static str, got: Object },

        #[snafu(display("Object with reference {reference} not found"))]
        ObjectNotFound {
            reference: crate::types::IndirectReference,
            source: crate::objects::Error,
        },

        #[snafu(display("Failed to convert array type"))]
        FailedArrayConvertion { source: crate::types::object::Error },

        #[snafu(display("Invalid rectangle format: expected {expected} coordinates, got {got}"))]
        InvalidRectangleFormat { expected: usize, got: usize },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Numeric, PdfString};

    #[snafu::report]
    #[test]
    fn test_array_builder() -> Result<()> {
        // Test 1: Basic array creation with generic()
        let array_obj = Object::Array(Array::from(vec![
            Object::Boolean(true),
            Object::Numeric(Numeric::Integer(42)),
            Object::String("test".into()),
        ]));

        let mut builder = ArrayBuilder::new(&array_obj);
        let result = builder.generic()?;

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], Object::Boolean(true));
        assert_eq!(result[1], Object::Numeric(Numeric::Integer(42)));
        assert_eq!(result[2], Object::String("test".into()));

        // Test 2: Array with of() method for type conversion
        let numeric_array = Object::Array(Array::from(vec![
            Object::Numeric(Numeric::Integer(1)),
            Object::Numeric(Numeric::Integer(2)),
            Object::Numeric(Numeric::Integer(3)),
        ]));

        let int_vec: Vec<i32> = ArrayBuilder::new(&numeric_array).of(|obj| obj.as_integer())?;

        assert_eq!(int_vec, vec![1, 2, 3]);

        // Test 3: Array with mixed types using of() with error handling
        let mixed_array = Object::Array(Array::from(vec![
            Object::Numeric(Numeric::Integer(10)),
            Object::Boolean(true), // This will cause conversion to fail
            Object::Numeric(Numeric::Integer(30)),
        ]));

        let result = ArrayBuilder::new(&mixed_array).of(|obj| obj.as_integer::<i32>());
        assert!(result.is_err(), "Test 3");

        // Test 4: Error case - not an array
        let not_array = Object::Boolean(true);
        let result = ArrayBuilder::new(&not_array).generic();
        assert!(result.is_err(), "Test 4");

        // Test 5: Empty array
        let empty_array = Object::Array(Array::from(vec![]));

        let empty_result = ArrayBuilder::new(&empty_array).generic()?;
        assert_eq!(empty_result.len(), 0);

        let empty_vec: Vec<i32> = ArrayBuilder::new(&empty_array).of(|obj| obj.as_integer())?;
        assert_eq!(empty_vec.len(), 0);

        // Test 6: Array with floating point numbers
        let float_array = Object::Array(Array::from(vec![
            Object::Numeric(Numeric::Real(1.5)),
            Object::Numeric(Numeric::Real(2.7)),
            Object::Numeric(Numeric::Real(3.1)),
        ]));

        let float_vec: Vec<f64> = ArrayBuilder::new(&float_array).of(|obj| obj.as_float())?;

        assert_eq!(float_vec, vec![1.5, 2.7, 3.1]);

        // Test 7: Array with boolean values
        let bool_array = Object::Array(Array::from(vec![
            Object::Boolean(true),
            Object::Boolean(false),
            Object::Boolean(true),
        ]));

        let bool_vec: Vec<bool> = ArrayBuilder::new(&bool_array).of(|obj| obj.as_bool())?;

        assert_eq!(bool_vec, vec![true, false, true]);

        // Test 8: Array with string values
        let string_array = Object::Array(Array::from(vec![
            Object::String("hello".into()),
            Object::String("world".into()),
        ]));

        let string_refs: Vec<PdfString> =
            ArrayBuilder::new(&string_array).of(|obj| obj.as_string().map(|s| s.clone()))?;

        assert_eq!(string_refs.len(), 2);
        assert_eq!(string_refs[0], PdfString::Literal(String::from("hello")));
        assert_eq!(string_refs[1], PdfString::Literal(String::from("world")));

        // Test 9: Array with name values
        let name_array = Object::Array(Array::from(vec![
            Object::Name("Type".into()),
            Object::Name("Font".into()),
        ]));

        let name_vec: Vec<String> =
            ArrayBuilder::new(&name_array).of(|obj| obj.as_name().map(|s| s.to_string()))?;

        assert_eq!(name_vec, vec![String::from("Type"), String::from("Font")]);

        // Test 10: Complex nested array (testing generic only)
        let nested_array = Object::Array(Array::from(vec![
            Object::Array(Array::from(vec![
                Object::Numeric(Numeric::Integer(1)),
                Object::Numeric(Numeric::Integer(2)),
            ])),
            Object::Boolean(false),
        ]));

        let nested_result = ArrayBuilder::new(&nested_array).generic()?;

        assert_eq!(nested_result.len(), 2);
        match &nested_result[0] {
            Object::Array(arr) => {
                assert_eq!(arr.len(), 2);
            }
            _ => panic!("Expected array at position 0"),
        }
        assert_eq!(nested_result[1], Object::Boolean(false));

        // Test 11: Array conversion with custom transformation
        let transform_array = Object::Array(Array::from(vec![
            Object::Numeric(Numeric::Integer(5)),
            Object::Numeric(Numeric::Integer(10)),
            Object::Numeric(Numeric::Integer(15)),
        ]));

        let doubled_vec: Vec<i32> = ArrayBuilder::new(&transform_array).of(|obj| {
            let num: i32 = obj.as_integer()?;
            Ok(num * 2) // Double each value
        })?;

        assert_eq!(doubled_vec, vec![10, 20, 30]);

        println!("All ArrayBuilder tests passed!");
        Ok(())
    }

    // Test for the Array type itself
    #[snafu::report]
    #[test]
    fn test_array_type() -> Result<()> {
        // Test Deref implementation
        let array = Array::from(vec![
            Object::Boolean(true),
            Object::Numeric(Numeric::Integer(42)),
        ]);

        assert_eq!(array.len(), 2);
        assert!(!array.is_empty());
        assert_eq!(array[0], Object::Boolean(true));

        // Test From implementations
        let vec_obj = vec![
            Object::String("test1".into()),
            Object::String("test2".into()),
        ];
        let array_from_vec: Array = vec_obj.into();
        assert_eq!(array_from_vec.len(), 2);

        let array_literal = [
            Object::Numeric(Numeric::Integer(1)),
            Object::Numeric(Numeric::Integer(2)),
            Object::Numeric(Numeric::Integer(3)),
        ];
        let array_from_array: Array = array_literal.into();
        assert_eq!(array_from_array.len(), 3);

        // Test Default
        let default_array = Array::default();
        assert!(default_array.is_empty());

        Ok(())
    }

    #[snafu::report]
    #[test]
    fn test_rectangle_creation() -> Result<()> {
        // Test normalized coordinates (already in correct order)
        let rect = Rectangle::new(10.0, 20.0, 100.0, 80.0);
        assert_eq!(rect.left(), 10.0);
        assert_eq!(rect.bottom(), 20.0);
        assert_eq!(rect.right(), 100.0);
        assert_eq!(rect.top(), 80.0);
        assert_eq!(rect.width(), 90.0);
        assert_eq!(rect.height(), 60.0);

        // Test unnormalized coordinates (should be normalized automatically)
        let rect = Rectangle::new(100.0, 80.0, 10.0, 20.0);
        assert_eq!(rect.left(), 10.0);
        assert_eq!(rect.bottom(), 20.0);
        assert_eq!(rect.right(), 100.0);
        assert_eq!(rect.top(), 80.0);

        // Test with negative coordinates
        let rect = Rectangle::new(-50.0, -30.0, 25.0, 15.0);
        assert_eq!(rect.left(), -50.0);
        assert_eq!(rect.bottom(), -30.0);
        assert_eq!(rect.right(), 25.0);
        assert_eq!(rect.top(), 15.0);
        assert_eq!(rect.width(), 75.0);
        assert_eq!(rect.height(), 45.0);

        Ok(())
    }

    #[snafu::report]
    #[test]
    fn test_array_builder_as_rectangle() -> Result<()> {
        // Test valid rectangle with integer coordinates
        let rect_array = Object::Array(Array::from(vec![
            Object::Numeric(Numeric::Integer(10)),
            Object::Numeric(Numeric::Integer(20)),
            Object::Numeric(Numeric::Integer(100)),
            Object::Numeric(Numeric::Integer(80)),
        ]));

        let rect = ArrayBuilder::new(&rect_array).rectangle()?;

        assert_eq!(rect.left(), 10.0);
        assert_eq!(rect.bottom(), 20.0);
        assert_eq!(rect.right(), 100.0);
        assert_eq!(rect.top(), 80.0);

        // Test valid rectangle with float coordinates
        let rect_array = Object::Array(Array::from(vec![
            Object::Numeric(Numeric::Real(10.5)),
            Object::Numeric(Numeric::Real(20.25)),
            Object::Numeric(Numeric::Real(100.75)),
            Object::Numeric(Numeric::Real(80.125)),
        ]));

        let rect = ArrayBuilder::new(&rect_array).rectangle()?;

        assert_eq!(rect.left(), 10.5);
        assert_eq!(rect.bottom(), 20.25);
        assert_eq!(rect.right(), 100.75);
        assert_eq!(rect.top(), 80.125);

        // Test rectangle with unnormalized coordinates (should be normalized)
        let rect_array = Object::Array(Array::from(vec![
            Object::Numeric(Numeric::Integer(100)),
            Object::Numeric(Numeric::Integer(80)),
            Object::Numeric(Numeric::Integer(10)),
            Object::Numeric(Numeric::Integer(20)),
        ]));

        let rect = ArrayBuilder::new(&rect_array).rectangle()?;

        assert_eq!(rect.left(), 10.0);
        assert_eq!(rect.bottom(), 20.0);
        assert_eq!(rect.right(), 100.0);
        assert_eq!(rect.top(), 80.0);

        // Test error: wrong number of elements
        let invalid_array = Object::Array(Array::from(vec![
            Object::Numeric(Numeric::Integer(10)),
            Object::Numeric(Numeric::Integer(20)),
            Object::Numeric(Numeric::Integer(100)),
            // Missing fourth coordinate
        ]));

        let result = ArrayBuilder::new(&invalid_array).rectangle();
        assert!(result.is_err(), "Test error: wrong number of elements");

        // Test error: too many elements
        let invalid_array = Object::Array(Array::from(vec![
            Object::Numeric(Numeric::Integer(10)),
            Object::Numeric(Numeric::Integer(20)),
            Object::Numeric(Numeric::Integer(100)),
            Object::Numeric(Numeric::Integer(80)),
            Object::Numeric(Numeric::Integer(90)), // Extra element
        ]));

        let result = ArrayBuilder::new(&invalid_array).rectangle();
        assert!(result.is_err(), "Test error: too many elements");

        // Test error: non-numeric elements
        let invalid_array = Object::Array(Array::from(vec![
            Object::Numeric(Numeric::Integer(10)),
            Object::Boolean(true), // Invalid type
            Object::Numeric(Numeric::Integer(100)),
            Object::Numeric(Numeric::Integer(80)),
        ]));

        let result = ArrayBuilder::new(&invalid_array).rectangle();
        assert!(result.is_err(), "Test error: non-numeric elements");

        // Test error: not an array
        let not_array = Object::Boolean(true);
        let result = ArrayBuilder::new(&not_array).rectangle();
        assert!(result.is_err(), "Test error: not an array");

        println!("All Rectangle tests passed!");
        Ok(())
    }
}
