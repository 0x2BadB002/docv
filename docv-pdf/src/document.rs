use std::{fs::File, path::PathBuf};

use memmap2::Mmap;
use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default)]
pub struct Document {
    size: u64,
    path: PathBuf,
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

        let _file = unsafe { Mmap::map(&file) }.with_context(|_| error::MmapSnafu {
            path: self.path.clone(),
        })?;

        // #[cfg(unix)]
        // {
        //     file.advise(Advice::Sequential)?; // Sequential access expected
        // }

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
    }
}
