use std::io::Read;

use flate2::read::ZlibDecoder;
use nom::Finish;

use crate::{
    Error, Result,
    parser::{DictionaryRecord, Numeric, Object, stream},
};

#[derive(Debug, Default)]
enum StreamFilterType {
    #[default]
    None,
    FlateDecode,
    PipeLine(Vec<StreamFilterType>),
}

pub fn process_stream(input: &[u8]) -> Result<(impl Iterator<Item = DictionaryRecord>, Vec<u8>)> {
    let (_, (records, data)) = stream(input).finish().map_err(|err| Error::Parser {
        message: err.code.description().to_string(),
    })?;

    let mut content_length = None;
    let mut records_rest = Vec::new();
    let mut filter = StreamFilterType::default();
    for record in records {
        match record.key.as_str() {
            "Length" => {
                content_length = Some(match record.value {
                    Object::Numeric(Numeric::Integer(length)) => length as usize,
                    _ => {
                        return Err(Error::InvalidObjectType {
                            expected: "Numeric::Integer".to_string(),
                            got: record.value,
                        });
                    }
                })
            }
            "Filter" => filter = process_filter(&record.value)?,
            _ => records_rest.push(record),
        }
    }

    let data = apply_filter(data, content_length, &filter)?;

    Ok((records_rest.into_iter(), data))
}

fn process_filter(filter: &Object) -> Result<StreamFilterType> {
    match filter {
        Object::Name(name) => match name.as_str() {
            "FlateDecode" => Ok(StreamFilterType::FlateDecode),
            _ => Err(Error::InvalidStreamFilterName(name.clone())),
        },
        Object::Array(pipeline) => Ok(StreamFilterType::PipeLine(
            pipeline
                .iter()
                .map(process_filter)
                .collect::<Result<Vec<_>>>()?,
        )),
        _ => Err(Error::InvalidObjectType {
            expected: "Name".to_string(),
            got: filter.clone(),
        }),
    }
}

fn apply_filter(
    data: &[u8],
    content_length: Option<usize>,
    filter: &StreamFilterType,
) -> Result<Vec<u8>> {
    match filter {
        StreamFilterType::None => Ok(data.to_vec()),
        StreamFilterType::FlateDecode => {
            let mut decoder = ZlibDecoder::new(data);
            let mut data =
                Vec::<u8>::with_capacity(content_length.ok_or(Error::InvalidStreamLength)?);

            decoder.read_to_end(&mut data)?;

            Ok(data)
        }
        StreamFilterType::PipeLine(filters) => {
            filters.iter().try_fold(data.to_vec(), |data, filter| {
                apply_filter(&data, content_length, filter)
            })
        }
    }
}
