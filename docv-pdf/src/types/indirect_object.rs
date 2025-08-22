use std::rc::Rc;

use crate::types::object::Object;

/// Represents a parsed PDF indirect object.
///
/// PDF indirect objects consist of:
/// 1. An object ID (positive integer)
/// 2. A generation number (non-negative integer)
/// 3. The `obj` keyword
/// 4. The object content
/// 5. The `endobj` keyword
///
/// Allows optional whitespace, comments, and end-of-line markers
/// between components.
#[derive(Debug, PartialEq, Clone)]
pub struct IndirectObject {
    pub id: usize,
    pub gen_id: usize,
    pub object: Rc<Object>,
}

/// Represents a parsed PDF indirect object reference.
///
/// PDF references consist of:
/// 1. An object ID (positive integer)
/// 2. A generation number (non-negative integer)
/// 3. The `R` keyword
///
/// Allows optional whitespace, comments, and end-of-line markers
/// between components.
#[derive(Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Clone, Default)]
pub struct IndirectReference {
    pub id: usize,
    pub gen_id: usize,
}
