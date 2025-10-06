use std::collections::BTreeMap;

use snafu::{OptionExt, ResultExt, Snafu, ensure};

use crate::{
    parser::{XrefObject, XrefTableSection, read_startxref, read_trailer, read_version, read_xref},
    structures::hash::Hash,
    structures::root::version::Version,
    types::{Dictionary, IndirectReference, Stream},
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default, Clone)]
pub struct Xref {
    prev: Option<u64>,
    xref_stm: Option<u64>,
    size: usize,
    version: Version,
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
        stream_ind: usize,
    },
}

#[derive(Debug, Clone)]
pub struct XrefMetadata {
    pub root_id: IndirectReference,
    pub version: Version,

    pub hash: Option<Hash>,
    pub info_id: Option<IndirectReference>,
}

impl Xref {
    pub fn find_entry<'a>(&'a self, ref_id: &IndirectReference) -> Option<&'a XrefEntry> {
        self.entries.get(ref_id)
    }

    pub fn has_more_tables(&self) -> bool {
        self.xref_stm.is_some() || self.prev.is_some()
    }

    pub fn read_startxref(&mut self, input: &[u8], filesize: usize) -> Result<u64> {
        let (_, version) = read_version(input).ok().context(error::ParseFile {
            section: "version",
            offset: 0usize,
        })?;
        self.version = Version::from_str(version).context(error::InvalidVersion)?;

        let offset = ((filesize as f64).log10().floor() + 1.0) as usize + 23;

        let start = filesize - offset;
        let (_, offset) = read_startxref(&input[start..])
            .ok()
            .context(error::ParseFile {
                section: "startxref",
                offset: start,
            })?;

        Ok(offset)
    }

    pub fn read_table(&mut self, input: &[u8], offset: u64) -> Result<XrefMetadata> {
        let start = offset as usize;
        let (remained, data) = read_xref(&input[start..]).ok().context(error::ParseFile {
            section: "xref",
            offset: start,
        })?;

        match data {
            XrefObject::Table(sections) => {
                self.parse_xref_table(sections)?;

                self.parse_trailer(remained)
            }
            XrefObject::Stream(mut stream) => {
                stream.process_filters().context(error::StreamProcessing)?;

                self.parse_xref_stream(stream)
            }
            XrefObject::IndirectStream(indirect_object) => {
                let mut stream = indirect_object
                    .as_stream()
                    .context(error::InvalidStream)?
                    .clone();

                stream.process_filters().context(error::StreamProcessing)?;

                self.parse_xref_stream(stream)
            }
        }
    }

    pub fn read_additional_table(&mut self, input: &[u8]) -> Result<()> {
        let offset = self
            .xref_stm
            .or(self.prev)
            .context(error::NoXRefAdditionalSources)?;

        self.read_table(input, offset)?;

        Ok(())
    }

    fn insert_entry(&mut self, key: IndirectReference, entry: XrefEntry) {
        if self.entries.contains_key(&key) {
            return;
        }

        let _ = self.entries.insert(key, entry);
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

                self.insert_entry(key, entry);
            }
        }
        Ok(())
    }

    fn parse_trailer(&mut self, input: &[u8]) -> Result<XrefMetadata> {
        let (_, trailer) = read_trailer(input).ok().context(error::ParseFile {
            section: "trailer",
            offset: 0usize,
        })?;

        self.xref_stm = trailer
            .get("XRefStm")
            .map(|object| object.as_integer())
            .transpose()
            .context(error::InvalidField { field: "XRefStm" })?;

        self.get_xref_data(&trailer)
    }

    fn get_xref_data(&mut self, data: &Dictionary) -> Result<XrefMetadata> {
        let size = data
            .get("Size")
            .context(error::FieldNotFound { field: "Size" })?
            .as_integer()
            .context(error::InvalidField { field: "Size" })?;

        let prev = data
            .get("Prev")
            .map(|object| object.as_integer())
            .transpose()
            .context(error::InvalidField { field: "Prev" })?;

        let file_hash = data
            .get("ID")
            .map(Hash::from_object)
            .transpose()
            .context(error::InvalidHash)?;

        let root_id = data
            .get("Root")
            .context(error::FieldNotFound { field: "Root" })?
            .as_indirect_ref()
            .cloned()
            .context(error::InvalidField { field: "Root" })?;

        let info_id = data
            .get("Info")
            .map(|object| object.as_indirect_ref().cloned())
            .transpose()
            .context(error::InvalidField { field: "Info" })?;

        // TODO: Support encrypt

        self.size = self.size.max(size);
        self.prev = prev;

        Ok(XrefMetadata {
            root_id,
            version: self.version.clone(),

            hash: file_hash,
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
            .context(error::FieldNotFound { field: "W" })?
            .as_array()
            .of(|obj| obj.as_integer())
            .context(error::InvalidArray { field: "W" })?;

        ensure!(
            w.len() == 3,
            error::InvalidXrefStreamWSize { size: w.len() }
        );

        let index = stream
            .dictionary
            .get("Index")
            .map(|object| object.as_array().generic())
            .transpose()
            .context(error::InvalidArray { field: "Index" })?
            .map(|array| {
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
            .context(error::InvalidField { field: "Index" })?
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

                self.insert_entry(key, entry);

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

                self.insert_entry(key, entry);

                Ok(())
            }
            2 => {
                let key = IndirectReference { id, gen_id: 0 };
                let entry = XrefEntry::OccupiedCompressed {
                    stream_id: entry_data[1],
                    stream_ind: entry_data[2],
                };

                self.insert_entry(key, entry);

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
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Failed to parse section {section}. Error at offset {offset}"))]
        ParseFile {
            section: &'static str,
            offset: usize,
        },

        #[snafu(display("Wrong version string format"))]
        InvalidVersion {
            source: crate::structures::root::version::Error,
        },

        #[snafu(display("Xref has no XRefStm or Prev instances"))]
        NoXRefAdditionalSources,

        #[snafu(display("Invalid object stream provided"))]
        InvalidStream { source: crate::types::object::Error },

        #[snafu(display("Error during stream processing"))]
        StreamProcessing { source: crate::types::stream::Error },

        #[snafu(display("Xref field `{field}` not found"))]
        FieldNotFound { field: &'static str },

        #[snafu(display("Wrong field {field} data format"))]
        InvalidField {
            field: &'static str,
            source: crate::types::object::Error,
        },

        #[snafu(display("Wrong field ID hash data format"))]
        InvalidHash {
            source: crate::structures::hash::Error,
        },

        #[snafu(display("Invalid array for field {field}"))]
        InvalidArray {
            field: &'static str,
            source: crate::types::array::Error,
        },

        #[snafu(display("Invalid Xref Stream `W` array size. Expected = 3, Got = {size}"))]
        InvalidXrefStreamWSize { size: usize },

        #[snafu(display(
            "Invalid Xref Stream entry type within binary data. Expected one of [0, 1, 2], Got = {entry_type}"
        ))]
        InvalidXrefStreamEntryType { entry_type: usize },
    }
}
