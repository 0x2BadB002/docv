use std::vec::IntoIter;

use nom::{
    IResult, ParseTo, Parser,
    branch::alt,
    bytes::complete::{tag, take, take_until},
    character::complete::digit1,
    combinator::{opt, value},
    multi::many0,
    sequence::{delimited, preceded, separated_pair, terminated},
};

use crate::parser::{
    DictionaryRecord, Object, dictionary, object,
    whitespace::eol,
    whitespace::{comment, whitespace},
};

pub enum Xref {
    Table(IntoIter<XrefTableSection>),
    ObjectStream(Object),
}

pub struct XrefTableSection {
    pub first_id: usize,
    pub length: usize,
    pub entries: IntoIter<XrefTableEntry>,
}

pub struct XrefTableEntry {
    pub offset: u64,
    pub gen_id: usize,
    pub occupied: bool,
}

pub fn startxref(input: &[u8]) -> IResult<&[u8], u64> {
    let value = digit1.map_opt(|res: &[u8]| res.parse_to());

    preceded(
        take_until("startxref"),
        delimited((tag("startxref"), eol), value, (eol, tag("%%EOF"))),
    )
    .parse(input)
}

pub fn xref(input: &[u8]) -> IResult<&[u8], Xref> {
    alt((xref_table.map(Xref::Table), object.map(Xref::ObjectStream))).parse(input)
}

pub fn trailer(input: &[u8]) -> IResult<&[u8], impl Iterator<Item = DictionaryRecord>> {
    let trailer = terminated(tag("trailer"), many0(alt((whitespace, eol, comment))));

    preceded(trailer, dictionary).parse(input)
}

fn xref_table(input: &[u8]) -> IResult<&[u8], IntoIter<XrefTableSection>> {
    let subsection = (
        separated_pair(
            digit1.map_opt(|res: &[u8]| res.parse_to()),
            tag(" "),
            digit1.map_opt(|res: &[u8]| res.parse_to()),
        ),
        many0(
            (
                take(10usize).map_opt(|res: &[u8]| res.parse_to()),
                value((), tag(" ")),
                take(5usize).map_opt(|res: &[u8]| res.parse_to()),
                value((), tag(" ")),
                alt((value(true, tag("n")), value(false, tag("f")))),
                opt(tag(" ")),
                eol,
            )
                .map(|(offset, _, gen_id, _, occupied, _, _)| XrefTableEntry {
                    offset,
                    gen_id,
                    occupied,
                }),
        ),
    )
        .map(|((first_id, length), entries)| XrefTableSection {
            first_id,
            length,
            entries: entries.into_iter(),
        });

    preceded((tag("xref"), eol), many0(subsection))
        .map(|res| res.into_iter())
        .parse(input)
}
