use std::{collections::BTreeMap, fs::File};

use memmap2::Mmap;
use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    parser::read_object,
    structures::{ObjectStream, Xref, XrefEntry, XrefMetadata},
    types::{IndirectReference, Object},
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Objects {
    file: Mmap,
    xref: Xref,

    object_streams: BTreeMap<usize, ObjectStream>,
}

impl Objects {
    pub fn from_file(file: File) -> Result<(Self, XrefMetadata)> {
        let file = unsafe { Mmap::map(&file) }.context(error::MmapSnafu)?;
        let mut xref = Xref::default();

        // #[cfg(unix)]
        // {
        //     file.advise(Advice::Sequential)?; // Sequential access expected
        // }

        let xref_offset = xref
            .read_startxref(&file, file.len())
            .context(error::ReadXrefSnafu)?;
        let metadata = xref
            .read_table(&file, xref_offset)
            .context(error::ReadXrefSnafu)?;

        Ok((
            Self {
                file,
                xref,
                object_streams: BTreeMap::default(),
            },
            metadata,
        ))
    }

    pub fn get_object(&mut self, object_reference: &IndirectReference) -> Result<Object> {
        let mut entry = self.xref.find_entry(object_reference);

        while entry.is_none() && self.xref.has_more_tables() {
            self.xref
                .read_additional_table(&self.file)
                .context(error::ReadXrefSnafu)?;

            entry = self.xref.find_entry(object_reference);
        }

        let entry = entry.with_context(|| error::EntryNotFoundSnafu {
            object: object_reference.clone(),
        })?;

        match *entry {
            XrefEntry::Free { .. } => Err(error::Error::EntryIsFree {
                object: object_reference.clone(),
            }
            .into()),
            XrefEntry::Occupied { offset } => {
                let object = read_object(&self.file[offset..])
                    .ok()
                    .context(error::ReadEntrySnafu)?;

                Ok(object)
            }
            XrefEntry::OccupiedCompressed {
                stream_id,
                stream_ind,
            } => {
                let stream = self.object_streams.get(&stream_id);

                match stream {
                    Some(stream) => {
                        let object = stream
                            .get_object_by_index(stream_ind)
                            .context(error::GetObjectFromStreamObjectSnafu)?;

                        Ok(object)
                    }
                    None => {
                        let object = self.get_object(&IndirectReference {
                            id: stream_id,
                            gen_id: 0,
                        })?;
                        let object = object.as_stream().context(error::ObjectSnafu)?.clone();
                        let stream = ObjectStream::from_stream(object)
                            .context(error::CreateObjectStreamSnafu)?;

                        let object = stream
                            .get_object_by_index(stream_ind)
                            .context(error::GetObjectFromStreamObjectSnafu)?;

                        self.object_streams.insert(stream_id, stream);

                        Ok(object)
                    }
                }
            }
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

        #[snafu(display("Invalid object type"))]
        Object { source: crate::types::ObjectError },

        #[snafu(display("Can't create ObjectStream from stream"))]
        CreateObjectStream {
            source: crate::structures::ObjectStreamError,
        },

        #[snafu(display("Failed to get object from ObjectStream"))]
        GetObjectFromStreamObject {
            source: crate::structures::ObjectStreamError,
        },
    }
}
