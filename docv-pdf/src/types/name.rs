use smol_str::SmolStr;

#[derive(Debug, PartialEq, Clone)]
pub struct Name {
    data: SmolStr,
}

impl<T: std::convert::Into<SmolStr>> From<T> for Name {
    fn from(value: T) -> Self {
        Self { data: value.into() }
    }
}

impl std::ops::Deref for Name {
    type Target = SmolStr;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
