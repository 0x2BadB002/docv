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
/// <<
///   /Type /Catalog
///   /Pages 2 0 R
///   /ViewerPreferences << /DisplayDocTitle true >>
/// >>
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Dictionary {
    records: BTreeMap<String, Object>,
}

impl<K: std::string::ToString> From<Vec<(K, Object)>> for Dictionary {
    fn from(value: Vec<(K, Object)>) -> Self {
        let value = value.into_iter().map(|(key, val)| (key.to_string(), val));

        Self {
            records: BTreeMap::from_iter(value),
        }
    }
}

impl<K: std::string::ToString, const N: usize> From<[(K, Object); N]> for Dictionary {
    fn from(value: [(K, Object); N]) -> Self {
        let value = value.map(|(key, val)| (key.to_string(), val));

        Self {
            records: BTreeMap::from(value),
        }
    }
}

impl std::ops::Deref for Dictionary {
    type Target = BTreeMap<String, Object>;

    fn deref(&self) -> &Self::Target {
        &self.records
    }
}
