use snafu::{ResultExt, Snafu, ensure};

use crate::types::Object;

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

/// Represents a file identifier hash used in PDF cross-reference streams.
///
/// In PDF files, the trailer contains a file identifier hash that uniquely identifies
/// the file. This hash is typically stored as an array of two byte strings:
/// - The first string (initial) is a permanent identifier based on the file's contents
///   at creation time
/// - The second string (current) is a changing identifier based on the file's current
///   contents
///
/// This structure is used to parse and represent these identifier hashes from
/// PDF objects.
#[derive(Debug, Clone)]
pub struct Hash {
    initial: Vec<u8>,
    current: Vec<u8>,
}

impl Hash {
    pub fn from_object(object: &Object) -> Result<Self> {
        let array = object.as_array().generic().context(error::Array)?;

        ensure!(
            array.len() == 2,
            error::InvalidArraySize {
                expected: 2usize,
                got: array.len()
            }
        );

        let initial = array[0]
            .as_string()
            .context(error::Object)?
            .as_bytes()
            .to_vec();
        let current = array[1]
            .as_string()
            .context(error::Object)?
            .as_bytes()
            .to_vec();

        Ok(Self { initial, current })
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let initial_hex: String = self.initial.iter().map(|b| format!("{:02x}", b)).collect();

        let current_hex: String = self.current.iter().map(|b| format!("{:02x}", b)).collect();

        write!(f, "{}:{}", initial_hex, current_hex)
    }
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Object conversion error"))]
        Object { source: crate::types::object::Error },

        #[snafu(display("Array conversion error"))]
        Array { source: crate::types::array::Error },

        #[snafu(display("Wrong array size. Expected = {expected}; Got = {got}"))]
        InvalidArraySize { expected: usize, got: usize },
    }
}
