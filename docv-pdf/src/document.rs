use std::{fs::File, path::PathBuf};

use snafu::{ResultExt, Snafu};

use crate::{
    objects::Objects,
    pages::Pages,
    structures::{
        hash::Hash,
        info::Info,
        root::{Root, version::Version},
    },
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Document {
    root: Root,
    info: Info,
    objects: Objects,

    size: u64,
    version: Version,
    hash: Option<Hash>,
}

impl Document {
    pub fn from_path(path: &PathBuf) -> crate::Result<Self> {
        let file = File::open(path)
            .with_context(|_| error::OpenFile { path: path.clone() })
            .map_err(|err| err.into())
            .context(crate::error::Document)?;

        let file_metadata = file
            .metadata()
            .context(error::Metadata)
            .map_err(|err| err.into())
            .context(crate::error::Document)?;

        let (mut objects, metadata) = Objects::from_file(file)
            .context(error::Objects)
            .map_err(|err| err.into())
            .context(crate::error::Document)?;

        let root_object = objects
            .get_object(&metadata.root_id)
            .context(error::Object {
                object: metadata.root_id,
            })
            .map_err(|err| err.into())
            .context(crate::error::Document)?;

        let root = Root::from_object(root_object, &mut objects)
            .context(error::Root)
            .map_err(|err| err.into())
            .context(crate::error::Document)?;

        let info = metadata
            .info_id
            .map(|object| -> Result<Info> {
                let object = objects
                    .get_object(&object)
                    .context(error::Object { object })?;

                Ok(Info::from_object(object).context(error::Info)?)
            })
            .transpose()
            .context(crate::error::Document)?;

        Ok(Document {
            root,
            info: info.unwrap_or_default(),
            objects,

            size: file_metadata.len(),
            version: metadata.version,
            hash: metadata.hash,
        })
    }

    pub fn info(&self) -> &Info {
        &self.info
    }

    pub fn version(&self) -> &Version {
        self.root.version.as_ref().unwrap_or(&self.version)
    }

    pub fn filesize(&self) -> u64 {
        self.size
    }

    pub fn hash(&self) -> Option<&Hash> {
        self.hash.as_ref()
    }

    /// Iterator over pages in a PDF document's page tree.
    ///
    /// The `Pages` struct provides an iterator that traverses the PDF page tree
    /// structure, resolving indirect references and flattening the hierarchy
    /// into a sequence of individual page objects.
    ///
    /// # Page Tree Structure
    /// PDF documents organize pages in a tree structure where:
    /// - The root node is a `/Pages` object
    /// - Intermediate nodes are also `/Pages` objects (containing `/Kids`)
    /// - Leaf nodes are `/Page` objects
    /// - Attributes can be inherited from parent pages nodes
    ///
    /// This iterator performs a depth-first traversal of this tree structure.
    ///
    /// # Usage
    /// ```
    /// use std::path::PathBuf;
    /// use docv_pdf::Document;
    ///
    /// let mut document = Document::from_path(&PathBuf::from("../example_files/report1.pdf")).unwrap();
    /// for page in document.pages() {
    ///     let page = page.unwrap();
    ///
    ///     // Process page...
    /// }
    /// ```
    ///
    /// # Note
    /// This iterator consumes and mutates the `Objects` store to resolve
    /// indirect references. It should not be used concurrently with other
    /// operations that modify the same objects store.
    pub fn pages<'a>(&'a mut self) -> Pages<'a> {
        Pages::new(&self.root.pages, &mut self.objects)
    }
}

mod error {
    use std::path::PathBuf;

    use snafu::Snafu;

    use crate::types::IndirectReference;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
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

        #[snafu(display("Failed to read root dictionary"))]
        Root {
            source: crate::structures::root::Error,
        },

        #[snafu(display("Failed to read info dictionary"))]
        Info {
            source: crate::structures::info::Error,
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

            let mut document = Document::from_path(&path)
                .with_whatever_context(|_| format!("Failed to open file {}", path.display()))?;

            let count = document.pages().count();

            let result = document
                .pages()
                .collect::<std::result::Result<Vec<_>, _>>()
                .with_whatever_context(|_| {
                    format!("Failed to iterate over pages for file {}", path.display())
                })?;

            assert_eq!(result.len(), count);
        }
        Ok(())
    }
}
