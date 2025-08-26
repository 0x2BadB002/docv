use std::{fs::File, path::PathBuf};

use memmap2::Mmap;
use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    info::Info,
    xref::{Xref, XrefEntry},
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DocumentHash {
    initial: Vec<u8>,
    current: Vec<u8>,
}

#[derive(Debug)]
pub struct Document {
    size: u64,
    file: Mmap,
    xref: Xref,
    info: Info,

    hash: Option<DocumentHash>,
}

impl DocumentHash {
    pub fn from_data(initial: Vec<u8>, current: Vec<u8>) -> Self {
        Self { initial, current }
    }
}

impl Document {
    pub fn from_path(path: &PathBuf) -> Result<Self> {
        let file =
            File::open(path).with_context(|_| error::OpenFileSnafu { path: path.clone() })?;
        let metadata = file
            .metadata()
            .with_context(|_| error::MetadataSnafu { path: path.clone() })?;

        let size = metadata.len();
        let file = unsafe { Mmap::map(&file) }.with_context(|_| error::MmapSnafu { path })?;

        // #[cfg(unix)]
        // {
        //     file.advise(Advice::Sequential)?; // Sequential access expected
        // }

        Ok(Self {
            size,
            file,

            xref: Xref::default(),
            info: Info::default(),
            hash: None,
        })
    }

    pub fn read_xref(&mut self) -> Result<()> {
        let metadata = self
            .xref
            .read(&self.file, self.size)
            .context(error::ReadXrefSnafu)?;

        self.hash = metadata.hash;
        if metadata.info_id.is_some() {
            let ref_id = metadata.info_id.as_ref().unwrap();
            let entry =
                self.xref
                    .find_entry(ref_id)
                    .with_context(|| error::EntryNotFoundSnafu {
                        object: ref_id.clone(),
                    })?;

            match entry {
                XrefEntry::Free { .. } => {}
                XrefEntry::Occupied { offset } => {
                    self.info
                        .read(&self.file, *offset)
                        .context(error::ReadInfoSnafu)?;
                }
                XrefEntry::OccupiedCompressed { .. } => {}
            }
        }

        Ok(())
    }
}

mod error {
    use std::path::PathBuf;

    use snafu::Snafu;

    use crate::types::IndirectReference;

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

        #[snafu(display("Failed to read xref table"))]
        ReadXref { source: crate::xref::Error },

        #[snafu(display("Failed to read info dictionary"))]
        ReadInfo { source: crate::info::Error },

        #[snafu(display("Failed to find offset for indirect object {object:?}"))]
        EntryNotFound { object: IndirectReference },
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
            let entry = example.whatever_context("Failed to directory entry")?;
            let path = entry.path();

            let mut document = Document::from_path(&path)
                .with_whatever_context(|_| format!("Failed to open file {}", path.display()))?;

            document
                .read_xref()
                .with_whatever_context(|_| format!("Failed to read file {}", path.display()))?;
        }
        Ok(())
    }
}
