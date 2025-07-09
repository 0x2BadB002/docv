use std::{fs::File, io::BufReader, path::PathBuf};

use memmap2::{Advice, Mmap};

use crate::Result;

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
        let file = File::open(&self.path)?;

        self.size = file.metadata()?.len();

        let file = unsafe { Mmap::map(&file)? };

        // #[cfg(unix)]
        // {
        //     file.advise(Advice::Sequential)?; // Sequential access expected
        // }

        Ok(())
    }
}
