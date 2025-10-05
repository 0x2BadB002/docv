use snafu::{ResultExt, Snafu};

use crate::types::Object;

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Hash {
    initial: Vec<u8>,
    current: Vec<u8>,
}

impl Hash {
    pub fn from_object(object: &Object) -> Result<Self> {
        let array = object
            .as_array()
            .generic()
            .context(error::InvalidArraySnafu)?;

        if array.len() != 2 {
            return Err(error::Error::InvalidArraySize {
                expected: 2,
                got: array.len(),
            }
            .into());
        }

        let initial = array[0]
            .as_string()
            .context(error::InvalidObjectSnafu)?
            .as_bytes()
            .to_vec();
        let current = array[1]
            .as_string()
            .context(error::InvalidObjectSnafu)?
            .as_bytes()
            .to_vec();

        Ok(Self { initial, current })
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut initial_hex = String::with_capacity(self.initial.len() * 2 + 2);
        for (i, el) in self.initial.iter().enumerate() {
            if i != 0 && i != 32 && i % 8 == 0 {
                initial_hex.push('-');
            }

            initial_hex += &format!("{el:#04x}")[2..];
        }

        let mut current_hex = String::with_capacity(self.initial.len() * 2 + 2);
        for (i, el) in self.current.iter().enumerate() {
            if i != 0 && i != 32 && i % 8 == 0 {
                current_hex.push('-');
            }

            current_hex += &format!("{el:#04x}")[2..];
        }

        write!(f, "{}:{}", initial_hex, current_hex)
    }
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Object conversion error"))]
        InvalidObject { source: crate::types::ObjectError },

        #[snafu(display("Array conversion error"))]
        InvalidArray { source: crate::types::array::Error },

        #[snafu(display("Wrong array size. Expected = {expected}; Got = {got}"))]
        InvalidArraySize { expected: usize, got: usize },
    }
}
