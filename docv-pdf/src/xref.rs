use std::{collections::BTreeMap, vec::IntoIter};

use nom::Finish;

use crate::{
    Error, Result,
    parser::{
        DictionaryRecord, IndirectReference, Numeric, Object, XrefTableSection, startxref, trailer,
        xref,
    },
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
            .map_err(|_| Error::Parser {
                message: "Failed to parse startxref".to_string(),
            })
    }

    fn read_table(&mut self, input: &[u8], _filesize: u64, offset: u64) -> Result<XrefMetadata> {
        let _startxref_size = 9 + ((offset as f64).log10().floor() + 1.0) as usize + 5;

        // let table_offset = filesize - offset - startxref_size as u64; // Approximate table size

        let (remained, data) =
            xref(&input[offset as usize..])
                .finish()
                .map_err(|_| Error::Parser {
                    message: "Failed to parse xref table".to_string(),
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
        struct XrefTrailerData {
            size: Option<usize>,
            prev: Option<u64>,
            file_hash: Option<PdfFileHash>,
            root_id: Option<XrefReference>,
            info_id: Option<XrefReference>,
        }

        let dictionary_data =
            trailer(input)
                .finish()
                .map(|(_, data)| data)
                .map_err(|_| Error::Parser {
                    message: "Failed to parse trailer".to_string(),
                })?;

        let mut size = None;
        let mut prev = None;
        let mut file_hash = None;
        for DictionaryRecord { key, value } in dictionary_data {
            match key {
                "Size" => {
                    size = match value {
                        Object::Numeric(Numeric::Integer(num)) => Some(num as usize),
                        _ => {
                            return Err(Error::InvalidObjectType {
                                expected: "Numeric".to_string(),
                                got: value,
                            });
                        }
                    };
                }
                "Prev" => {
                    prev = match value {
                        Object::Numeric(Numeric::Integer(num)) => Some(num as u64),
                        _ => {
                            return Err(Error::InvalidObjectType {
                                expected: "Numeric".to_string(),
                                got: value,
                            });
                        }
                    };
                }
                "ID" => {
                    let array = match value {
                        Object::Array(ids) => ids,
                        _ => {
                            return Err(Error::InvalidObjectType {
                                expected: "Numeric".to_string(),
                                got: value,
                            });
                        }
                    };
                    if array.len() != 2 {
                        return Err(Error::InvalidXrefIDSize(array.len()));
                    }

                    data.file_hash = Some(PdfFileHash {
                        initial: array[0].clone(),
                        current: array[1].clone(),
                    });

                    Ok(())
                }
                "Root" => {
                    let reference = parse_indirect_reference(object)?;

                    data.root_id = Some(XrefReference {
                        id: reference.0,
                        generation: reference.1,
                    });

                    Ok(())
                }
                "Info" => {
                    let reference = parse_indirect_reference(object)?;

                    data.info_id = Some(XrefReference {
                        id: reference.0,
                        generation: reference.1,
                    });

                    Ok(())
                }
                // TODO: Support encrypt
                _ => Err(Error::UnexpectedDictionaryField(key.to_string())),
            }
        }

        let new_size = data.size.ok_or(Error::NoXrefSizeProvided)?;
        if new_size > self.size {
            self.size = new_size;
        }

        self.prev = data.prev;

        Ok(XrefMetadata {
            file_hash: data.file_hash,
            root_id: data.root_id.ok_or(Error::NoRootObjectProvided)?,
            info_id: data.info_id,
        })
    }

    // fn parse_xref_stream(
    //     &mut self,
    //     token: Pair<Rule>,
    //     chunks: Vec<Vec<u8>>,
    // ) -> Result<XrefMetadata> {
    //     #[derive(Default, Debug)]
    //     struct XrefStreamData {
    //         w: Vec<usize>,
    //         index: Option<Vec<(usize, usize)>>,
    //         size: Option<usize>,
    //         prev: Option<u64>,
    //         file_hash: Option<PdfFileHash>,
    //
    //         root_id: Option<XrefReference>,
    //         info_id: Option<XrefReference>,
    //     }
    //
    //     let data = parse_stream(
    //         token,
    //         &chunks,
    //         "XRef",
    //         |data: &mut XrefStreamData, key, object| match key {
    //             "W" => {
    //                 data.w = parse_array(object, |obj| {
    //                     obj.as_str().parse::<usize>().map_err(Error::IntConv)
    //                 })?;
    //
    //                 let size = data.w.len();
    //                 if size != 3 {
    //                     Err(Error::InvalidXrefStreamWSize(size))
    //                 } else {
    //                     Ok(())
    //                 }
    //             }
    //             "Size" => {
    //                 let size = parse_numeric(object)?;
    //
    //                 if data.index.is_none() {
    //                     data.index = Some(vec![(0, size)]);
    //                 }
    //
    //                 data.size = Some(size);
    //
    //                 Ok(())
    //             }
    //             "Index" => {
    //                 let array = parse_array(object, |obj| {
    //                     obj.as_str().parse::<usize>().map_err(Error::IntConv)
    //                 })?
    //                 .chunks(2)
    //                 .map(|chunk| (chunk[0], chunk[1]))
    //                 .collect();
    //
    //                 data.index = Some(array);
    //
    //                 Ok(())
    //             }
    //             "Prev" => {
    //                 let offset = parse_numeric(object)?;
    //
    //                 data.prev = Some(offset);
    //
    //                 Ok(())
    //             }
    //             "Root" => {
    //                 let reference = parse_indirect_reference(object)?;
    //
    //                 data.root_id = Some(XrefReference {
    //                     id: reference.0,
    //                     generation: reference.1,
    //                 });
    //
    //                 Ok(())
    //             }
    //             "Info" => {
    //                 let reference = parse_indirect_reference(object)?;
    //
    //                 data.info_id = Some(XrefReference {
    //                     id: reference.0,
    //                     generation: reference.1,
    //                 });
    //
    //                 Ok(())
    //             }
    //             "ID" => {
    //                 let array = parse_array(object, |el| {
    //                     let hex_string = parse_hex_string(el)?;
    //                     Ok(hex_string)
    //                 })?;
    //
    //                 if array.len() != 2 {
    //                     return Err(Error::InvalidXrefIDSize(array.len()));
    //                 }
    //
    //                 data.file_hash = Some(PdfFileHash {
    //                     initial: array[0].clone(),
    //                     current: array[1].clone(),
    //                 });
    //
    //                 Ok(())
    //             }
    //             // TODO: Support encrypt
    //             _ => Err(Error::UnexpectedDictionaryField(key.to_string())),
    //         },
    //         |data, buff| {
    //             let _table_size = data.size.ok_or(Error::NoXrefSizeProvided)?;
    //             let entry_size = data.w.iter().sum();
    //
    //             let current_id = data
    //                 .index
    //                 .as_ref()
    //                 .unwrap()
    //                 .iter()
    //                 .flat_map(|(first, last)| *first..=*last);
    //
    //             buff.chunks_exact(entry_size)
    //                 .zip(current_id)
    //                 .try_for_each(|(entry, id)| {
    //                     let mut entry_data = [1, 0, 0];
    //
    //                     data.w
    //                         .iter()
    //                         .zip(entry_data.iter_mut())
    //                         .fold(0, |pos, (size, data)| {
    //                             if *size == 0 {
    //                                 return pos;
    //                             }
    //
    //                             *data = entry[pos..(pos + size)]
    //                                 .iter()
    //                                 .fold(0u64, |res, byte| res << 8 | (*byte as u64));
    //                             pos + size
    //                         });
    //
    //                     match entry_data[0] {
    //                         0 => Ok(()),
    //                         1 => {
    //                             self.entries.insert(
    //                                 XrefReference {
    //                                     id,
    //                                     generation: entry_data[2] as usize,
    //                                 },
    //                                 XrefEntry {
    //                                     offset: entry_data[1],
    //                                     occupied: true,
    //                                 },
    //                             );
    //                             Ok(())
    //                         }
    //                         2 => Ok(()),
    //                         _ => Err(Error::UnexpectedXrefStreamEntryType(entry_data[0])),
    //                     }
    //                 })
    //         },
    //     )?;
    //
    //     let new_size = data.size.ok_or(Error::NoXrefSizeProvided)?;
    //     if new_size > self.size {
    //         self.size = new_size;
    //     }
    //
    //     self.prev = data.prev;
    //
    //     Ok(XrefMetadata {
    //         file_hash: data.file_hash,
    //         root_id: data.root_id.ok_or(Error::NoRootObjectProvided)?,
    //         info_id: data.info_id,
    //     })
    // }
}
