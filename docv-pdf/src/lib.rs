use std::{fs::File, io::BufReader, path::PathBuf};

mod error;
mod info;
mod parser;
mod xref;

pub use crate::error::{Error, Result};

#[derive(Debug, Default)]
pub struct Document {
    size: u64,
    path: PathBuf,

    xref_table: xref::XrefTable,
    metadata: xref::XrefMetadata,

    info: info::Info,
}

impl Document {
    pub fn from_path(path: PathBuf) -> Self {
        Self {
            path,

            ..Default::default()
        }
    }

    pub fn read(&mut self) -> Result<()> {
        let file = File::open(&self.path)?;

        self.size = file.metadata()?.len();

        let mut reader = BufReader::new(file);

        self.metadata = self.xref_table.read(&mut reader, self.size)?;

        self.xref_table.read_prev_table(&mut reader, self.size)?;

        if let Some(info_id) = self.metadata.info_id.as_ref() {
            if let Some(offset) = self.xref_table.find_offset(info_id) {
                self.info.read(&mut reader, offset)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{fs, sync::LazyLock};

    static EXAMPLES: LazyLock<PathBuf> = LazyLock::new(|| {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.pop();
        dir.push("examples");
        dir
    });

    #[test]
    fn read_example_files() {
        for example in fs::read_dir(EXAMPLES.clone()).expect("Failed to read examples dir.") {
            let example = example.unwrap();
            eprintln!("Reading file {}...", example.path().display());

            let mut pdf_file = Document::from_path(example.path());

            pdf_file
                .read()
                .map_err(|err| {
                    eprintln!("Working with file {}", example.path().display());
                    eprintln!("{}", err.to_string());
                    err
                })
                .expect("Failed to read file");
        }
    }
}
