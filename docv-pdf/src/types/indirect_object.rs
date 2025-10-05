use std::{fmt::Display, ops::Deref, sync::Arc};

use crate::types::object::Object;

/// Represents a parsed PDF indirect object.
///
/// PDF indirect objects consist of:
/// - An object ID (positive integer)
/// - A generation number (non-negative integer)
/// - The `obj` keyword
/// - The object content
/// - The `endobj` keyword
///
/// Allows optional whitespace, comments, and end-of-line markers
/// between components.
#[derive(Debug, PartialEq, Clone)]
pub struct IndirectObject {
    pub id: usize,
    pub gen_id: usize,
    object: Arc<Object>,
}

/// Represents a parsed PDF indirect object reference.
///
/// PDF references consist of:
/// - An object ID (positive integer)
/// - A generation number (non-negative integer)
/// - The `R` keyword
///
/// Allows optional whitespace, comments, and end-of-line markers
/// between components.
#[derive(Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Clone, Default, Copy)]
pub struct IndirectReference {
    pub id: usize,
    pub gen_id: usize,
}

impl IndirectObject {
    pub fn new(id: usize, gen_id: usize, object: Object) -> Self {
        Self {
            id,
            gen_id,
            object: Arc::new(object),
        }
    }

    pub fn get_object(&self) -> &Object {
        &self.object
    }
}

impl Deref for IndirectObject {
    type Target = Object;

    fn deref(&self) -> &Self::Target {
        &self.object
    }
}

impl Display for IndirectReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} R", self.id, self.gen_id)
    }
}
