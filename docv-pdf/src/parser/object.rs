use nom::{IResult, Parser, branch::alt};

use crate::{
    parser::{
        array::array,
        boolean::boolean,
        dictionary::dictionary,
        indirect_object::{indirect_object, indirect_reference},
        name::name,
        null::null,
        numeric::numeric,
        stream::stream,
        string::pdf_string,
    },
    types::Object,
};

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
        stream.map(Object::Stream),
        array.map(Object::Array),
        dictionary.map(Object::Dictionary),
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
