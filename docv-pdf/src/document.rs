use std::{fs::File, path::PathBuf};

use memmap2::Mmap;
use snafu::{ResultExt, Snafu};

use crate::xref::XrefTable;

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default, Clone)]
#[allow(dead_code)] // NOTE: Maybe later will be removed
pub struct DocumentHash {
    initial: Vec<u8>,
    current: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Document {
    size: u64,
    path: PathBuf,
    xref: XrefTable,

    hash: Option<DocumentHash>,
}

impl DocumentHash {
    pub fn from_data(initial: Vec<u8>, current: Vec<u8>) -> Self {
        Self { initial, current }
    }
}

impl Document {
    pub fn from_path(path: PathBuf) -> Self {
        Self {
            path,

            ..Default::default()
        }
    }

    pub fn read(&mut self) -> Result<()> {
        let file = File::open(&self.path).with_context(|_| error::OpenFileSnafu {
            path: self.path.clone(),
        })?;

        let metadata = file.metadata().with_context(|_| error::MetadataSnafu {
            path: self.path.clone(),
        })?;

        self.size = metadata.len();

        let file = unsafe { Mmap::map(&file) }.with_context(|_| error::MmapSnafu {
            path: self.path.clone(),
        })?;

        // #[cfg(unix)]
        // {
        //     file.advise(Advice::Sequential)?; // Sequential access expected
        // }

        let metadata = self
            .xref
            .read(&file, self.size)
            .with_context(|_| error::ReadXrefSnafu {
                path: self.path.clone(),
            })?;

        self.hash = metadata.hash;

        Ok(())
    }
}

mod error {
    use std::path::PathBuf;

    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Failed to open file: {}", path.display()))]
        OpenFile {
            path: PathBuf,
            source: std::io::Error,
        },

        #[snafu(display("Failed to get metadata for file: {}", path.display()))]
        Metadata {
            path: PathBuf,
            source: std::io::Error,
        },

        #[snafu(display("Failed to create mmap for file: {}", path.display()))]
        Mmap {
            path: PathBuf,
            source: std::io::Error,
        },

        #[snafu(display("Failed to read xref table for file: {}", path.display()))]
        ReadXref {
            path: PathBuf,
            source: crate::xref::Error,
        },
    }
}

#[cfg(test)]
mod test {
    use snafu::Whatever;

    use super::*;
    use std::{fs, sync::LazyLock};

    static EXAMPLES: LazyLock<PathBuf> = LazyLock::new(|| {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.pop();
        dir.push("example_files");
        dir
    });

    #[snafu::report]
    #[test]
    fn read_example_files() -> std::result::Result<(), Whatever> {
        for example in
            fs::read_dir(EXAMPLES.clone()).whatever_context("Failed to read directory")?
        {
            let example = example.whatever_context("Failed to directory entry")?;

            let mut pdf_file = Document::from_path(example.path());

            pdf_file.read().whatever_context("Failed to read file")?;
        }
        Ok(())
    }
}
