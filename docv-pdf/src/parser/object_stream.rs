use core::str;

use nom::{
    Finish, Parser,
    branch::alt,
    character::complete::digit1,
    error::Error,
    multi::{count, many0},
    sequence::terminated,
};

use super::whitespace::{comment, eol, whitespace};

pub fn header_array(input: &[u8], n: usize) -> Result<Vec<(usize, usize)>, Error<&[u8]>> {
    count(
        (
            terminated(
                digit1.map_res(|s| str::from_utf8(s)),
                many0(alt((whitespace, comment, eol))),
            )
            .map_res(|s| s.parse::<usize>()),
            terminated(
                digit1.map_res(|s| str::from_utf8(s)),
                many0(alt((whitespace, comment, eol))),
            )
            .map_res(|s| s.parse::<usize>()),
        ),
        n,
    )
    .parse(input)
    .finish()
    .map(|(_, res)| res)
}
