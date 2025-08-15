use std::{collections::BTreeMap, vec::IntoIter};

use nom::Finish;

use crate::{
    Error, Result,
    parser::{DictionaryRecord, IndirectReference, XrefTableSection, startxref, trailer, xref},
    process_stream::process_stream,
};

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
        self.entries.get(ref_id).map(|entry| entry.offset)
    }

    fn read_startxref(&mut self, input: &[u8], filesize: u64) -> Result<u64> {
        let offset = ((filesize as f64).log10().floor() + 1.0) as usize + 23;

        startxref(&input[offset..])
            .finish()
            .map(|(_, res)| res)
            .map_err(|err| Error::Parser {
                message: err.code.description().to_string(),
            })
    }

    fn read_table(&mut self, input: &[u8], _filesize: u64, offset: u64) -> Result<XrefMetadata> {
        let _startxref_size = 9 + ((offset as f64).log10().floor() + 1.0) as usize + 5;

        // let table_offset = filesize - offset - startxref_size as u64; // Approximate table size

        let (remained, data) =
            xref(&input[offset as usize..])
                .finish()
                .map_err(|err| Error::Parser {
                    message: err.code.description().to_string(),
                })?;

        match data {
            crate::parser::Xref::Table(sections) => {
                self.parse_xref_table(sections)?;

                self.parse_trailer(remained)
            }
            crate::parser::Xref::ObjectStream(object) => todo!(),
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
        #[derive(Default)]
        struct TrailerDict {
            size: Option<usize>,
            prev: Option<u64>,
            file_hash: Option<PdfFileHash>,
            root_id: Option<IndirectReference>,
            info_id: Option<IndirectReference>,
        }

        let dictionary_data = trailer(input)
            .finish()
            .map(|(_, data)| data)
            .map_err(|err| Error::Parser {
                message: err.code.description().to_string(),
            })?;
        let mut dict = TrailerDict::default();

        for DictionaryRecord { key, value } in dictionary_data {
            match key.as_str() {
                "Size" => dict.size = Some(value.as_integer()? as usize),
                "Prev" => dict.prev = Some(value.as_integer()? as u64),
                "ID" => {
                    let array = value.as_array()?;
                    if array.len() < 2 {
                        return Err(Error::InvalidXrefIDSize(array.len()));
                    }
                    dict.file_hash = Some(PdfFileHash {
                        initial: array[0].as_bytes()?.to_vec(),
                        current: array[1].as_bytes()?.to_vec(),
                    });
                }
                "Root" => dict.root_id = Some(value.as_indirect_ref()?.clone()),
                "Info" => dict.info_id = Some(value.as_indirect_ref()?.clone()),
                // TODO: Support encrypt
                _ => return Err(Error::UnknownDictionaryField(key.to_string())),
            }
        }

        let new_size = dict.size.ok_or(Error::NoXrefSizeProvided)?;

        if new_size > self.size {
            self.size = new_size;
        }
        self.prev = dict.prev;

        Ok(XrefMetadata {
            file_hash: dict.file_hash,
            root_id: dict.root_id.ok_or(Error::NoRootObjectProvided)?,
            info_id: dict.info_id,
        })
    }

    fn parse_xref_stream(&mut self, input: &[u8]) -> Result<XrefMetadata> {
        #[derive(Default)]
        struct XrefStreamDict {
            w: Option<Vec<usize>>,
            index: Option<Vec<(usize, usize)>>,
            size: Option<usize>,
            prev: Option<u64>,
            file_hash: Option<PdfFileHash>,
            root_id: Option<IndirectReference>,
            info_id: Option<IndirectReference>,
        }

        let (records, data) = process_stream(input)?;
        let mut dict = XrefStreamDict::default();

        for DictionaryRecord { key, value } in records {
            match key.as_str() {
                "W" => {
                    let w = value
                        .as_array()?
                        .iter()
                        .map(|el| el.as_integer().map(|n| n as usize))
                        .collect::<Result<Vec<_>>>()?;
                    if w.len() != 3 {
                        return Err(Error::InvalidXrefStreamWSize(w.len()));
                    }
                    dict.w = Some(w);
                }
                "Size" => dict.size = Some(value.as_integer()? as usize),
                "Index" => {
                    let array = value.as_array()?;
                    dict.index = Some(
                        array
                            .chunks_exact(2)
                            .map(|chunk| {
                                let first = chunk[0].as_integer()? as usize;
                                let second = chunk[1].as_integer()? as usize;
                                Ok((first, second))
                            })
                            .collect::<Result<Vec<_>>>()?,
                    );
                }
                "Prev" => dict.prev = Some(value.as_integer()? as u64),
                "Root" => dict.root_id = Some(value.as_indirect_ref()?.clone()),
                "Info" => dict.info_id = Some(value.as_indirect_ref()?.clone()),
                "ID" => {
                    let array = value.as_array()?;
                    if array.len() < 2 {
                        return Err(Error::InvalidXrefIDSize(array.len()));
                    }
                    dict.file_hash = Some(PdfFileHash {
                        initial: array[0].as_bytes()?.to_vec(),
                        current: array[1].as_bytes()?.to_vec(),
                    });
                }
                // TODO: Support encrypt
                _ => return Err(Error::UnknownDictionaryField(key.to_string())),
            }
        }

        let w = dict.w.ok_or(Error::InvalidXrefStreamWSize(0))?;
        let size = dict.size.ok_or(Error::NoXrefSizeProvided)?;
        let index = dict.index.unwrap_or_else(|| vec![(0, size)]);
        let entry_size = w.iter().sum();

        let current_id = index.iter().flat_map(|(first, last)| *first..=*last);

        data.chunks_exact(entry_size)
            .zip(current_id)
            .try_for_each(|(entry, id)| {
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
                    _ => Err(Error::UnexpectedXrefStreamEntryType(entry_data[0])),
                }
            })?;

        if size > self.size {
            self.size = size;
        }
        self.prev = dict.prev;

        Ok(XrefMetadata {
            file_hash: dict.file_hash,
            root_id: dict.root_id.ok_or(Error::NoRootObjectProvided)?,
            info_id: dict.info_id,
        })
    }
}
