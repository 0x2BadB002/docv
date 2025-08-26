use chrono::{DateTime, FixedOffset};
use nom::Finish;
use snafu::{OptionExt, ResultExt, Snafu};

use crate::parser::indirect_object;

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default)]
pub struct Info {
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    keywords: Option<String>,
    creator: Option<String>,
    producer: Option<String>,
    creation_date: Option<DateTime<FixedOffset>>,
    mod_date: Option<DateTime<FixedOffset>>,
    trapped: Trap,
    other: Vec<(String, String)>,
}

#[derive(Debug, Default)]
pub enum Trap {
    True,
    False,
    #[default]
    Unknown,
}

impl Info {
    pub fn read(&mut self, input: &[u8], offset: usize) -> Result<()> {
        let (_, info) = indirect_object(&input[offset..])
            .finish()
            .ok()
            .context(error::ParseSnafu { offset })?;
        let info = info.as_dictionary().context(error::NotDictionarySnafu)?;

        for (key, value) in info.records.iter() {
            match key.as_str() {
                "Title" => {
                    self.title = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidFieldSnafu { field: key.clone() })?
                            .as_str()
                            .context(error::PdfStringSnafu)?
                            .to_string(),
                    )
                }
                "Author" => {
                    self.author = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidFieldSnafu { field: key.clone() })?
                            .as_str()
                            .context(error::PdfStringSnafu)?
                            .to_string(),
                    )
                }
                "Subject" => {
                    self.subject = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidFieldSnafu { field: key.clone() })?
                            .as_str()
                            .context(error::PdfStringSnafu)?
                            .to_string(),
                    )
                }
                "Keywords" => {
                    self.keywords = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidFieldSnafu { field: key.clone() })?
                            .as_str()
                            .context(error::PdfStringSnafu)?
                            .to_string(),
                    )
                }
                "Creator" => {
                    self.creator = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidFieldSnafu { field: key.clone() })?
                            .as_str()
                            .context(error::PdfStringSnafu)?
                            .to_string(),
                    )
                }
                "Producer" => {
                    self.producer = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidFieldSnafu { field: key.clone() })?
                            .as_str()
                            .context(error::PdfStringSnafu)?
                            .to_string(),
                    )
                }
                "CreationDate" => {
                    self.creation_date = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidFieldSnafu { field: key.clone() })?
                            .to_date()
                            .context(error::PdfStringSnafu)?,
                    )
                }
                "ModDate" => {
                    self.mod_date = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidFieldSnafu { field: key.clone() })?
                            .to_date()
                            .context(error::PdfStringSnafu)?,
                    )
                }
                "Trapped" => {
                    let value = value
                        .as_string()
                        .with_context(|_| error::InvalidFieldSnafu { field: key.clone() })?
                        .as_str()
                        .context(error::PdfStringSnafu)?;

                    self.trapped = match value {
                        "True" => Trap::True,
                        "False" => Trap::False,
                        "Unknown" => Trap::Unknown,
                        _ => {
                            return Err(error::Error::UnexpectedTrapValue {
                                value: value.to_string(),
                            }
                            .into());
                        }
                    }
                }
                _ => self.other.push((
                    key.to_string(),
                    value
                        .as_string()
                        .with_context(|_| error::InvalidFieldSnafu { field: key.clone() })?
                        .as_str()
                        .context(error::PdfStringSnafu)?
                        .to_string(),
                )),
            }
        }

        Ok(())
    }

    pub fn title(&self) -> Option<&String> {
        self.title.as_ref()
    }

    pub fn author(&self) -> Option<&String> {
        self.author.as_ref()
    }

    pub fn subject(&self) -> Option<&String> {
        self.subject.as_ref()
    }

    pub fn keywords(&self) -> Option<&String> {
        self.keywords.as_ref()
    }

    pub fn creator(&self) -> Option<&String> {
        self.creator.as_ref()
    }

    pub fn producer(&self) -> Option<&String> {
        self.producer.as_ref()
    }

    pub fn creation_date(&self) -> Option<DateTime<FixedOffset>> {
        self.creation_date
    }

    pub fn mod_date(&self) -> Option<DateTime<FixedOffset>> {
        self.mod_date
    }

    pub fn trapped(&self) -> &Trap {
        &self.trapped
    }
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Failed to parse info object. Error at offset {offset}"))]
        Parse { offset: usize },

        #[snafu(display("Parsed object is not dictionary"))]
        NotDictionary { source: crate::types::ObjectError },

        #[snafu(display("Wrong field {field} data format"))]
        InvalidField {
            field: String,
            source: crate::types::ObjectError,
        },

        #[snafu(display("Unexpected Trapping value encountered. Value = {value}"))]
        UnexpectedTrapValue { value: String },

        #[snafu(display("Error while working with pdf string"))]
        PdfString { source: crate::types::StringError },
    }
}
