use snafu::{ResultExt, Snafu};

use crate::types::Object;

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Hash {
    initial: Vec<u8>,
    current: Vec<u8>,
}

impl Hash {
    pub fn from_object(object: &Object) -> Result<Self> {
        let array = object.as_array().context(error::InvalidObjectSnafu)?;

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

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Object conversion error"))]
        InvalidObject { source: crate::types::ObjectError },

        #[snafu(display("Wrong array size. Expected = {expected}; Got = {got}"))]
        InvalidArraySize { expected: usize, got: usize },
    }
}
