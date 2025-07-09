use std::collections::BTreeMap;
use std::io::Seek;
use std::vec;

use pest::iterators::Pair;

use crate::parser::{
    Rule, parse_array, parse_dictionary, parse_hex_string, parse_indirect_reference, parse_numeric,
    parse_startxref, parse_stream, parse_string, parse_xref, process_bytes,
};
use crate::{Error, Result};

#[derive(Debug, Default, Clone)]
pub struct XrefTable {
    entries: BTreeMap<XrefReference, XrefEntry>,
    size: usize,
    prev: Option<u64>,
}

#[derive(Debug, Eq, Hash, PartialEq, PartialOrd, Ord, Clone, Default)]
pub struct XrefReference {
    id: usize,
    generation: usize,
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
pub struct XrefEntry {
    offset: u64,
    occupied: bool,
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
pub struct XrefMetadata {
    pub file_hash: Option<PdfFileHash>,

    pub root_id: XrefReference,
    pub info_id: Option<XrefReference>,
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone)]
pub struct PdfFileHash {
    initial: Vec<u8>,
    current: Vec<u8>,
}

impl XrefTable {
    pub fn read<T>(&mut self, reader: &mut T, filesize: u64) -> Result<XrefMetadata>
    where
        T: Seek,
        T: std::io::Read,
    {
        let offset = self.read_startxref(reader, filesize)?;
        self.read_table(reader, filesize, offset)
    }

    pub fn read_prev_table<T>(&mut self, reader: &mut T, filesize: u64) -> Result<()>
    where
        T: Seek,
        T: std::io::Read,
    {
        if self.prev.is_some() {
            self.read_table(reader, filesize, self.prev.unwrap())?;
        }

        Ok(())
    }

    pub fn find_offset(&self, ref_id: &XrefReference) -> Option<u64> {
        self.entries.get(ref_id).map(|entry| entry.offset)
    }

    fn read_startxref<T>(&mut self, reader: &mut T, filesize: u64) -> Result<u64>
    where
        T: Seek,
        T: std::io::Read,
    {
        let offset = ((filesize as f64).log10().floor() + 1.0) as usize + 23;

        reader.seek(std::io::SeekFrom::End(-(offset as i64)))?;

        let mut buff = vec![0u8; offset];
        reader.read_exact(&mut buff)?;
        let buff = String::from_utf8(buff)?;

        let token = parse_startxref(&buff)?;
        let token = token
            .into_inner()
            .next()
            .ok_or_else(|| Error::InvalidXref(buff.to_string()))?;

        match token.as_rule() {
            Rule::last_xref_pos => Ok(token.as_str().parse()?),
            _ => Err(Error::InvalidXref(token.to_string())),
        }
    }

    fn read_table<T>(&mut self, reader: &mut T, filesize: u64, offset: u64) -> Result<XrefMetadata>
    where
        T: Seek,
        T: std::io::Read,
    {
        let startxref_size = 9 + ((offset as f64).log10().floor() + 1.0) as usize + 5;

        reader.seek(std::io::SeekFrom::Start(offset))?;

        let table_offset = filesize - offset - startxref_size as u64;
        let mut buff = vec![0u8; table_offset as usize];
        reader.read_exact(&mut buff)?;
        let (buff, chunks) = process_bytes(buff)?;

        let token = parse_xref(&buff)?;

        let mut token = token.into_inner();
        let table = token
            .next()
            .ok_or_else(|| Error::InvalidXref(buff.to_string()))?;

        match table.as_rule() {
            Rule::xref_old => {
                self.parse_xref_table(table)?;

                let trailer = token
                    .next()
                    .ok_or_else(|| Error::InvalidTrailer(buff.to_string()))?;

                self.parse_trailer(trailer)
            }
            Rule::indirect_definition => {
                let mut token = table.into_inner();

                let _ = token.next();
                let _ = token.next();

                let stream = token.next().unwrap();
                self.parse_xref_stream(stream, chunks)
            }
            _ => Err(Error::InvalidXref(token.to_string())),
        }
    }

    fn parse_xref_table(&mut self, token: Pair<Rule>) -> Result<()> {
        for subsection in token.into_inner() {
            let mut subsection_inner = subsection.into_inner();

            let first_id = subsection_inner.next().unwrap().as_str().parse::<usize>()?;
            let expected_count = subsection_inner.next().unwrap().as_str().parse::<usize>()?;
            let mut count = 0;

            for (i, entries) in subsection_inner.enumerate() {
                let mut token = entries.into_inner();

                let offset = token
                    .next()
                    .ok_or(Error::InvalidXref("Expected offset".to_string()))?
                    .as_str()
                    .parse::<u64>()?;

                let generation = token
                    .next()
                    .ok_or(Error::InvalidXref("Expected gen. number".to_string()))?
                    .as_str()
                    .parse::<usize>()?;

                let occupied = token
                    .next()
                    .ok_or(Error::InvalidXref("Expected occupied flag".to_string()))?
                    .as_str()
                    .eq("n");

                let entry = XrefEntry { offset, occupied };

                self.entries.insert(
                    XrefReference {
                        id: first_id + i,
                        generation,
                    },
                    entry,
                );

                count += 1;
            }
            if count > expected_count {
                return Err(Error::InvalidXrefTableSize(expected_count, count));
            }
        }
        Ok(())
    }

    fn parse_trailer(&mut self, token: Pair<Rule>) -> Result<XrefMetadata> {
        #[derive(Default)]
        struct XrefTrailerData {
            size: Option<usize>,
            prev: Option<u64>,
            file_hash: Option<PdfFileHash>,
            root_id: Option<XrefReference>,
            info_id: Option<XrefReference>,
        }

        let data = parse_dictionary(token, |data: &mut XrefTrailerData, key, object| match key {
            "Size" => {
                let size = parse_numeric(object)?;

                data.size = Some(size);

                Ok(())
            }
            "Prev" => {
                let offset = parse_numeric(object)?;

                data.prev = Some(offset);

                Ok(())
            }
            "ID" => {
                let array = parse_array(object, |el| {
                    let hex_string = parse_string(el)?;
                    Ok(hex_string)
                })?;

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
        })?;

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

    fn parse_xref_stream(
        &mut self,
        token: Pair<Rule>,
        chunks: Vec<Vec<u8>>,
    ) -> Result<XrefMetadata> {
        #[derive(Default, Debug)]
        struct XrefStreamData {
            w: Vec<usize>,
            index: Option<Vec<(usize, usize)>>,
            size: Option<usize>,
            prev: Option<u64>,
            file_hash: Option<PdfFileHash>,

            root_id: Option<XrefReference>,
            info_id: Option<XrefReference>,
        }

        let data = parse_stream(
            token,
            &chunks,
            "XRef",
            |data: &mut XrefStreamData, key, object| match key {
                "W" => {
                    data.w = parse_array(object, |obj| {
                        obj.as_str().parse::<usize>().map_err(Error::IntConv)
                    })?;

                    let size = data.w.len();
                    if size != 3 {
                        Err(Error::InvalidXrefStreamWSize(size))
                    } else {
                        Ok(())
                    }
                }
                "Size" => {
                    let size = parse_numeric(object)?;

                    if data.index.is_none() {
                        data.index = Some(vec![(0, size)]);
                    }

                    data.size = Some(size);

                    Ok(())
                }
                "Index" => {
                    let array = parse_array(object, |obj| {
                        obj.as_str().parse::<usize>().map_err(Error::IntConv)
                    })?
                    .chunks(2)
                    .map(|chunk| (chunk[0], chunk[1]))
                    .collect();

                    data.index = Some(array);

                    Ok(())
                }
                "Prev" => {
                    let offset = parse_numeric(object)?;

                    data.prev = Some(offset);

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
                "ID" => {
                    let array = parse_array(object, |el| {
                        let hex_string = parse_hex_string(el)?;
                        Ok(hex_string)
                    })?;

                    if array.len() != 2 {
                        return Err(Error::InvalidXrefIDSize(array.len()));
                    }

                    data.file_hash = Some(PdfFileHash {
                        initial: array[0].clone(),
                        current: array[1].clone(),
                    });

                    Ok(())
                }
                // TODO: Support encrypt
                _ => Err(Error::UnexpectedDictionaryField(key.to_string())),
            },
            |data, buff| {
                let _table_size = data.size.ok_or(Error::NoXrefSizeProvided)?;
                let entry_size = data.w.iter().sum();

                let current_id = data
                    .index
                    .as_ref()
                    .unwrap()
                    .iter()
                    .flat_map(|(first, last)| *first..=*last);

                buff.chunks_exact(entry_size)
                    .zip(current_id)
                    .try_for_each(|(entry, id)| {
                        let mut entry_data = [1, 0, 0];

                        data.w
                            .iter()
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
                                    XrefReference {
                                        id,
                                        generation: entry_data[2] as usize,
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
                    })
            },
        )?;

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
}
