use std::fs::File;

use memmap2::Mmap;
use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    parser::read_object,
    structures::{Xref, XrefEntry, XrefMetadata},
    types::{IndirectReference, Object},
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Objects {
    file: Mmap,
    xref: Xref,
}

impl Objects {
    pub fn from_file(file: File) -> Result<(Self, XrefMetadata)> {
        let file = unsafe { Mmap::map(&file) }.context(error::MmapSnafu)?;
        let mut xref = Xref::default();

        // #[cfg(unix)]
        // {
        //     file.advise(Advice::Sequential)?; // Sequential access expected
        // }

        let metadata = xref.read(&file, file.len()).context(error::ReadXrefSnafu)?;

        Ok((Self { file, xref }, metadata))
    }

    pub fn get_object(&mut self, object_reference: &IndirectReference) -> Result<Object> {
        let mut entry = self.xref.find_entry(object_reference);

        while entry.is_none() && self.xref.has_prev_table() {
            self.xref
                .read_prev_table(&self.file)
                .context(error::ReadXrefSnafu)?;

            entry = self.xref.find_entry(object_reference);
        }

        let entry = entry.with_context(|| error::EntryNotFoundSnafu {
            object: object_reference.clone(),
        })?;

        match entry {
            XrefEntry::Free { .. } => Err(error::Error::EntryIsFree {
                object: object_reference.clone(),
            }
            .into()),
            XrefEntry::Occupied { offset } => {
                let object = read_object(&self.file[(*offset)..])
                    .ok()
                    .context(error::ReadEntrySnafu)?;

                Ok(object)
            }
            XrefEntry::OccupiedCompressed { .. } => Ok(Object::Null),
        }
    }
}

mod error {
    use snafu::Snafu;

    use crate::types::IndirectReference;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Failed to create mmap"))]
        Mmap { source: std::io::Error },

        #[snafu(display("Failed to read xref table"))]
        ReadXref {
            source: crate::structures::XrefError,
        },

        #[snafu(display("Failed to read info dictionary"))]
        ReadEntry,

        #[snafu(display("Failed to find indirect object {object:?}"))]
        EntryNotFound { object: IndirectReference },

        #[snafu(display("Entry for indirect object {object:?} is free"))]
        EntryIsFree { object: IndirectReference },
    }
}
