use std::collections::BTreeMap;

use crate::types::object::Object;

/// Represents a PDF dictionary object containing key-value pairs.
///
/// PDF dictionaries are fundamental data structures that map name keys
/// (PDF name objects) to arbitrary PDF object values. They are used
/// throughout PDF specifications to organize document structure and metadata.
///
/// # Structure
/// - Keys are PDF names (always starting with '/')
/// - Values can be any valid PDF object type
/// - Implemented using BTreeMap for ordered storage and efficient lookups
///
/// # Examples
/// ```
/// <<
///   /Type /Catalog
///   /Pages 2 0 R
///   /ViewerPreferences << /DisplayDocTitle true >>
/// >>
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct Dictionary {
    pub records: BTreeMap<String, Object>,
}

impl Dictionary {
    /// Retrieves a reference to an object associated with the given key.
    ///
    /// The key should be a PDF name without the leading '/' character,
    /// as the internal storage normalizes names by removing the prefix.
    ///
    /// # Arguments
    /// * `key` - The dictionary key to look up (without leading '/')
    ///
    /// # Returns
    /// - `Some(&Object)` if the key exists in the dictionary
    /// - `None` if the key is not present
    ///
    /// # Example
    /// ```
    /// let dict = Dictionary { ... };
    /// if let Some(obj) = dict.get("Type") {
    ///     // Handle the object
    /// }
    /// ```
    pub fn get(&self, key: &str) -> Option<&Object> {
        self.records.get(key)
    }
}

impl<const N: usize> From<[(String, Object); N]> for Dictionary {
    fn from(value: [(String, Object); N]) -> Self {
        Self {
            records: BTreeMap::from(value),
        }
    }
}
