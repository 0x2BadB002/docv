use crate::types::Object;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Array {
    data: Vec<Object>,
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
