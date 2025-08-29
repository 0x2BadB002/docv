use std::{fs::File, path::PathBuf};

use snafu::{ResultExt, Snafu};

use crate::{
    objects::Objects,
    structures::{Hash, Info},
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Document {
    info: Info,
    objects: Objects,

    size: u64,
    hash: Option<Hash>,
}

impl Document {
    pub fn from_path(path: &PathBuf) -> Result<Self> {
        let file =
            File::open(path).with_context(|_| error::OpenFileSnafu { path: path.clone() })?;

        let file_metadata = file.metadata().context(error::MetadataSnafu)?;

        let (mut objects, metadata) = Objects::from_file(file).context(error::ObjectsSnafu)?;

        let info = metadata
            .info_id
            .map(|object| -> Result<Info> {
                let object = objects
                    .get_object(&object)
                    .with_context(|_| error::ObjectSnafu { object })?;

                Ok(Info::from_object(object).context(error::InfoSnafu)?)
            })
            .transpose()?;

        Ok(Self {
            info: info.unwrap_or_default(),
            objects,

            size: file_metadata.len(),
            hash: metadata.hash,
        })
    }

    pub fn info(&self) -> &Info {
        &self.info
    }

    pub fn filesize(&self) -> u64 {
        self.size
    }

    pub fn hash(&self) -> Option<&Hash> {
        self.hash.as_ref()
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

        #[snafu(display("Failed to get metadata"))]
        Metadata { source: std::io::Error },

        #[snafu(display("Failed to get objects"))]
        Objects { source: crate::objects::Error },

        #[snafu(display("Failed to get object {object}"))]
        Object {
            object: IndirectReference,
            source: crate::objects::Error,
        },

        #[snafu(display("Failed to read info dictionary"))]
        Info {
            source: crate::structures::InfoError,
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
            let entry = example.whatever_context("Failed to directory entry")?;
            let path = entry.path();

            let _document = Document::from_path(&path)
                .with_whatever_context(|_| format!("Failed to open file {}", path.display()))?;
        }
        Ok(())
    }
}
