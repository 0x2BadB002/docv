use std::{collections::BTreeMap, vec::IntoIter};

use nom::Finish;
use snafu::{OptionExt, ResultExt, Snafu};

use crate::{
    parser::{Xref, XrefTableSection, startxref, trailer, xref},
    types::{Dictionary, IndirectReference, Stream},
};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default, Clone)]
pub struct XrefTable {
    entries: BTreeMap<IndirectReference, XrefEntry>,
    size: usize,
    prev: Option<u64>,
}

#[derive(Debug, Default, Clone)]
pub struct XrefEntry {
    offset: u64,
    occupied: bool,
}

#[derive(Debug, Default, Clone)]
pub struct XrefMetadata {
    pub file_hash: Option<PdfFileHash>,

    pub root_id: IndirectReference,
    pub info_id: Option<IndirectReference>,
}

#[derive(Debug, Default, Clone)]
pub struct PdfFileHash {
    initial: Vec<u8>,
    current: Vec<u8>,
}

impl XrefTable {
    pub fn read(&mut self, input: &[u8], filesize: u64) -> Result<XrefMetadata> {
        let offset = self.read_startxref(input, filesize)?;
        self.read_table(input, filesize, offset)
    }

    pub fn read_prev_table(&mut self, input: &[u8], filesize: u64) -> Result<()> {
        if self.prev.is_some() {
            self.read_table(input, filesize, self.prev.unwrap())?;
        }

        Ok(())
    }

    pub fn find_offset(&self, ref_id: &IndirectReference) -> Option<u64> {
        self.entries
            .get(ref_id)
            .filter(|entry| entry.occupied)
            .map(|entry| entry.offset)
    }

    fn read_startxref(&mut self, input: &[u8], filesize: u64) -> Result<u64> {
        let offset = ((filesize as f64).log10().floor() + 1.0) as usize + 23;

        let start = (filesize as usize) - offset;
        let (_, offset) = startxref(&input[start..])
            .finish()
            .ok()
            .context(error::ParseFileSnafu { offset: start })?;

        Ok(offset)
    }

    fn read_table(&mut self, input: &[u8], _filesize: u64, offset: u64) -> Result<XrefMetadata> {
        let _startxref_size = 9 + ((offset as f64).log10().floor() + 1.0) as usize + 5;

        // let table_offset = filesize - offset - startxref_size as u64; // Approximate table size

        let start = offset as usize;
        let (remained, data) = xref(&input[start..])
            .finish()
            .ok()
            .context(error::ParseFileSnafu { offset: start })?;

        match data {
            Xref::Table(sections) => {
                self.parse_xref_table(sections)?;

                self.parse_trailer(remained)
            }
            Xref::ObjectStream(mut stream) => {
                stream
                    .process_filters()
                    .context(error::InvalidStreamSnafu)?;

                self.parse_xref_stream(stream)
            }
        }
    }

    fn parse_xref_table(&mut self, sections: IntoIter<XrefTableSection>) -> Result<()> {
        for section in sections {
            for (i, parsed_entry) in section.entries.enumerate() {
                let entry = XrefEntry {
                    offset: parsed_entry.offset,
                    occupied: parsed_entry.occupied,
                };

                self.entries.insert(
                    IndirectReference {
                        id: section.first_id + i,
                        gen_id: parsed_entry.gen_id,
                    },
                    entry,
                );
            }
        }
        Ok(())
    }

    fn parse_trailer(&mut self, input: &[u8]) -> Result<XrefMetadata> {
        let (_, trailer) = trailer(input)
            .finish()
            .ok()
            .context(error::ParseFileSnafu { offset: 0usize })?;

        self.get_xref_data(&trailer)
    }

    fn get_xref_data(&mut self, data: &Dictionary) -> Result<XrefMetadata> {
        let size = data
            .get("Size")
            .context(error::NoXrefSizeSnafu)?
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
            .map(|object| {
                let array = object
                    .as_array()
                    .with_context(|_| error::InvalidFieldSnafu {
                        field: "ID".to_string(),
                    })?;
                if array.len() != 2 {
                    return Err(error::Error::InvalidXrefIDSize { size: array.len() });
                }
                let initial = array[0]
                    .as_bytes()
                    .with_context(|_| error::InvalidFieldSnafu {
                        field: "ID".to_string(),
                    })?
                    .to_vec();
                let current = array[1]
                    .as_bytes()
                    .with_context(|_| error::InvalidFieldSnafu {
                        field: "ID".to_string(),
                    })?
                    .to_vec();

                Ok(PdfFileHash { initial, current })
            })
            .transpose()?;

        let root_id = data
            .get("Root")
            .context(error::NoXrefRootSnafu)?
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
            file_hash,
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
            .context(error::NoXrefStreamWSnafu)?
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
                    .fold(0u64, |res, byte| res << 8 | (*byte as u64));
                pos + size
            });

        match entry_data[0] {
            0 => Ok(()),
            1 => {
                self.entries.insert(
                    IndirectReference {
                        id,
                        gen_id: entry_data[2] as usize,
                    },
                    XrefEntry {
                        offset: entry_data[1],
                        occupied: true,
                    },
                );
                Ok(())
            }
            2 => Ok(()),
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
        #[snafu(display("Parser error at offset {}", offset))]
        ParseFile { offset: usize },

        #[snafu(display("Xref field `Size` not found"))]
        NoXrefSize,

        #[snafu(display("Xref field `Root` not found"))]
        NoXrefRoot,

        #[snafu(display("Wrong field {field} data format"))]
        InvalidField {
            field: String,
            source: crate::types::ObjectError,
        },

        #[snafu(display("Invalid Xref `ID` array size. Expected = 2, Got = {size}"))]
        InvalidXrefIDSize { size: usize },

        #[snafu(display("Xref stream field `W` not found"))]
        NoXrefStreamW,

        #[snafu(display("Invalid Xref Stream `W` array size. Expected = 3, Got = {size}"))]
        InvalidXrefStreamWSize { size: usize },

        #[snafu(display(
            "Invalid Xref Stream entry type within binary data. Expected one of [0, 1, 2], Got = {entry_type}"
        ))]
        InvalidXrefStreamEntryType { entry_type: u64 },

        #[snafu(display("Error during stream processing"))]
        InvalidStream { source: crate::types::StreamError },
    }
}
