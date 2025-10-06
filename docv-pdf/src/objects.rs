use std::{collections::BTreeMap, fs::File};

use memmap2::Mmap;
use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    parser::read_object,
    structures::object_stream::ObjectStream,
    structures::xref::{Xref, XrefEntry, XrefMetadata},
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
        let file = unsafe { Mmap::map(&file) }.context(error::Mmap)?;
        let mut xref = Xref::default();

        // #[cfg(unix)]
        // {
        //     file.advise(Advice::Sequential)?; // Sequential access expected
        // }

        let xref_offset = xref
            .read_startxref(&file, file.len())
            .context(error::ReadXref)?;
        let metadata = xref
            .read_table(&file, xref_offset)
            .context(error::ReadXref)?;

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
                .context(error::ReadXref)?;

            entry = self.xref.find_entry(object_reference);
        }

        let entry = entry.context(error::EntryNotFound {
            object: *object_reference,
        })?;

        match *entry {
            XrefEntry::Free { .. } => Err(error::Error::EntryIsFree {
                object: *object_reference,
            }
            .into()),
            XrefEntry::Occupied { offset } => {
                let object = read_object(&self.file[offset..])
                    .ok()
                    .context(error::ReadEntry)?;

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
                            .context(error::GetObjectFromStreamObject)?;

                        Ok(object)
                    }
                    None => {
                        let object = self.get_object(&IndirectReference {
                            id: stream_id,
                            gen_id: 0,
                        })?;
                        let object = object.as_stream().cloned().context(error::Object)?;
                        let stream =
                            ObjectStream::from_stream(object).context(error::CreateObjectStream)?;

                        let object = stream
                            .get_object_by_index(stream_ind)
                            .context(error::GetObjectFromStreamObject)?;

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
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Failed to create mmap"))]
        Mmap { source: std::io::Error },

        #[snafu(display("Failed to read xref table"))]
        ReadXref {
            #[snafu(source(from(crate::structures::xref::Error, Box::new)))]
            source: Box<crate::structures::xref::Error>,
        },

        #[snafu(display("Failed to read info dictionary"))]
        ReadEntry,

        #[snafu(display("Failed to find indirect object {object}"))]
        EntryNotFound { object: IndirectReference },

        #[snafu(display("Entry for indirect object {object} is free"))]
        EntryIsFree { object: IndirectReference },

        #[snafu(display("Invalid object type"))]
        Object { source: crate::types::object::Error },

        #[snafu(display("Can't create ObjectStream from stream"))]
        CreateObjectStream {
            source: crate::structures::object_stream::Error,
        },

        #[snafu(display("Failed to get object from ObjectStream"))]
        GetObjectFromStreamObject {
            source: crate::structures::object_stream::Error,
        },
    }
}
