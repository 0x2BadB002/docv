use std::collections::BTreeMap;

use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    parser::{XrefObject, XrefTableSection, read_startxref, read_trailer, read_xref},
    structures::Hash,
    types::{Dictionary, IndirectReference, Stream},
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default, Clone)]
pub struct Xref {
    prev: Option<u64>,
    size: usize,
    entries: BTreeMap<IndirectReference, XrefEntry>,
}

#[derive(Debug, Clone)]
pub enum XrefEntry {
    Free {
        #[allow(dead_code)]
        next_id: usize,
    },
    Occupied {
        offset: usize,
    },
    OccupiedCompressed {
        stream_id: usize,
        stream_offset_id: usize,
    },
}

#[derive(Debug, Default, Clone)]
pub struct XrefMetadata {
    pub hash: Option<Hash>,
    pub root_id: IndirectReference,
    pub info_id: Option<IndirectReference>,
}

impl Xref {
    pub fn find_entry<'a>(&'a self, ref_id: &IndirectReference) -> Option<&'a XrefEntry> {
        self.entries.get(ref_id)
    }

    pub fn has_prev_table(&self) -> bool {
        self.prev.is_some()
    }

    pub fn read_startxref(&mut self, input: &[u8], filesize: usize) -> Result<u64> {
        let offset = ((filesize as f64).log10().floor() + 1.0) as usize + 23;

        let start = filesize - offset;
        let (_, offset) =
            read_startxref(&input[start..])
                .ok()
                .with_context(|| error::ParseFileSnafu {
                    section: "startxref".to_string(),
                    offset: start,
                })?;

        Ok(offset)
    }

    pub fn read_table(&mut self, input: &[u8], offset: u64) -> Result<XrefMetadata> {
        let start = offset as usize;
        let (remained, data) =
            read_xref(&input[start..])
                .ok()
                .with_context(|| error::ParseFileSnafu {
                    section: "xref".to_string(),
                    offset: start,
                })?;

        match data {
            XrefObject::Table(sections) => {
                self.parse_xref_table(sections)?;

                self.parse_trailer(remained)
            }
            XrefObject::Stream(mut stream) => {
                stream
                    .process_filters()
                    .context(error::StreamProcessingSnafu)?;

                self.parse_xref_stream(stream)
            }
            XrefObject::IndirectStream(indirect_object) => {
                let mut stream = indirect_object
                    .as_stream()
                    .with_context(|_| error::InvalidStreamSnafu)?
                    .clone();

                stream
                    .process_filters()
                    .context(error::StreamProcessingSnafu)?;

                self.parse_xref_stream(stream)
            }
        }
    }

    pub fn read_prev_table(&mut self, input: &[u8]) -> Result<()> {
        self.read_table(input, self.prev.context(error::NoPrevXrefSnafu)?)?;

        Ok(())
    }

    fn parse_xref_table(&mut self, sections: Vec<XrefTableSection>) -> Result<()> {
        for section in sections.iter() {
            for (i, parsed_entry) in section.entries.iter().enumerate() {
                let key = IndirectReference {
                    id: section.first_id + i,
                    gen_id: parsed_entry.gen_id,
                };
                let entry = if parsed_entry.occupied {
                    XrefEntry::Occupied {
                        offset: parsed_entry.offset,
                    }
                } else {
                    XrefEntry::Free {
                        next_id: section.first_id + i,
                    }
                };

                self.entries.insert(key, entry);
            }
        }
        Ok(())
    }

    fn parse_trailer(&mut self, input: &[u8]) -> Result<XrefMetadata> {
        let (_, trailer) = read_trailer(input)
            .ok()
            .with_context(|| error::ParseFileSnafu {
                section: "trailer".to_string(),
                offset: 0usize,
            })?;

        self.get_xref_data(&trailer)
    }

    fn get_xref_data(&mut self, data: &Dictionary) -> Result<XrefMetadata> {
        let size = data
            .get("Size")
            .with_context(|| error::FieldNotFoundSnafu {
                field: "Size".to_string(),
            })?
            .as_integer()
            .with_context(|_| error::InvalidFieldSnafu {
                field: "Size".to_string(),
            })?;

        let prev = data
            .get("Prev")
            .map(|object| object.as_integer())
            .transpose()
            .with_context(|_| error::InvalidFieldSnafu {
                field: "Prev".to_string(),
            })?;

        let file_hash = data
            .get("ID")
            .map(|object| Hash::from_object(object).context(error::InvalidHashSnafu))
            .transpose()?;

        let root_id = data
            .get("Root")
            .with_context(|| error::FieldNotFoundSnafu {
                field: "Root".to_string(),
            })?
            .as_indirect_ref()
            .cloned()
            .with_context(|_| error::InvalidFieldSnafu {
                field: "Root".to_string(),
            })?;

        let info_id = data
            .get("Info")
            .map(|object| object.as_indirect_ref().cloned())
            .transpose()
            .with_context(|_| error::InvalidFieldSnafu {
                field: "Info".to_string(),
            })?;

        // TODO: Support encrypt
        if size > self.size {
            self.size = size;
        }
        self.prev = prev;

        Ok(XrefMetadata {
            hash: file_hash,
            root_id,
            info_id,
        })
    }

    fn parse_xref_stream(&mut self, stream: Stream) -> Result<XrefMetadata> {
        let metadata = self.get_xref_data(&stream.dictionary)?;

        self.extract_xref_stream_data(stream)?;

        Ok(metadata)
    }

    fn extract_xref_stream_data(&mut self, stream: Stream) -> Result<()> {
        let w = stream
            .dictionary
            .get("W")
            .with_context(|| error::FieldNotFoundSnafu {
                field: "W".to_string(),
            })?
            .as_array()
            .with_context(|_| error::InvalidFieldSnafu {
                field: "W".to_string(),
            })?
            .iter()
            .map(|el| el.as_integer())
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|_| error::InvalidFieldSnafu {
                field: "W".to_string(),
            })?;
        if w.len() != 3 {
            return Err(error::Error::InvalidXrefStreamWSize { size: w.len() }.into());
        }

        let index = stream
            .dictionary
            .get("Index")
            .map(|object| {
                let array = object.as_array()?;
                array
                    .chunks_exact(2)
                    .map(|chunk| {
                        let first = chunk[0].as_integer()?;
                        let second = chunk[1].as_integer()?;
                        Ok((first, second))
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()
            })
            .transpose()
            .with_context(|_| error::InvalidFieldSnafu {
                field: "Index".to_string(),
            })?
            .unwrap_or_else(|| vec![(0, self.size)]);

        let entry_size = w.iter().sum();
        let current_id = index.iter().flat_map(|(first, last)| *first..=*last);

        stream
            .data
            .chunks_exact(entry_size)
            .zip(current_id)
            .try_for_each(|(entry, id)| self.parse_stream_entry(&w, entry, id))?;

        Ok(())
    }

    fn parse_stream_entry(&mut self, w: &[usize], entry: &[u8], id: usize) -> Result<()> {
        let mut entry_data = [1, 0, 0];

        w.iter()
            .zip(entry_data.iter_mut())
            .fold(0, |pos, (size, data)| {
                if *size == 0 {
                    return pos;
                }

                *data = entry[pos..(pos + size)]
                    .iter()
                    .fold(0usize, |res, byte| res << 8 | (*byte as usize));
                pos + size
            });

        match entry_data[0] {
            0 => {
                let key = IndirectReference {
                    id,
                    gen_id: entry_data[2],
                };
                let entry = XrefEntry::Free {
                    next_id: entry_data[1],
                };

                self.entries.insert(key, entry);

                Ok(())
            }
            1 => {
                let key = IndirectReference {
                    id,
                    gen_id: entry_data[2],
                };
                let entry = XrefEntry::Occupied {
                    offset: entry_data[1],
                };

                self.entries.insert(key, entry);

                Ok(())
            }
            2 => {
                let key = IndirectReference { id, gen_id: 0 };
                let entry = XrefEntry::OccupiedCompressed {
                    stream_id: entry_data[1],
                    stream_offset_id: entry_data[2],
                };

                self.entries.insert(key, entry);

                Ok(())
            }
            _ => Err(error::Error::InvalidXrefStreamEntryType {
                entry_type: entry_data[0],
            }
            .into()),
        }
    }
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Failed to parse section {section}. Error at offset {offset}"))]
        ParseFile { section: String, offset: usize },

        #[snafu(display("Xref has no prev instances"))]
        NoPrevXref,

        #[snafu(display("Invalid object stream provided"))]
        InvalidStream { source: crate::types::ObjectError },

        #[snafu(display("Error during stream processing"))]
        StreamProcessing { source: crate::types::StreamError },

        #[snafu(display("Xref field `{field}` not found"))]
        FieldNotFound { field: String },

        #[snafu(display("Wrong field {field} data format"))]
        InvalidField {
            field: String,
            source: crate::types::ObjectError,
        },

        #[snafu(display("Wrong field ID hash data format"))]
        InvalidHash {
            source: crate::structures::HashError,
        },

        #[snafu(display("Invalid Xref Stream `W` array size. Expected = 3, Got = {size}"))]
        InvalidXrefStreamWSize { size: usize },

        #[snafu(display(
            "Invalid Xref Stream entry type within binary data. Expected one of [0, 1, 2], Got = {entry_type}"
        ))]
        InvalidXrefStreamEntryType { entry_type: usize },
    }
}
