use chrono::{DateTime, FixedOffset};
use snafu::{ResultExt, Snafu};

use crate::types::Object;

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default)]
pub struct Info {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<DateTime<FixedOffset>>,
    pub mod_date: Option<DateTime<FixedOffset>>,
    pub trapped: Trap,
    pub other: Vec<(String, String)>,
}

#[derive(Debug, Default)]
pub enum Trap {
    True,
    False,
    #[default]
    Unknown,
}

impl Info {
    pub fn populate_from_dictionary(&mut self, dictionary: Object) -> Result<()> {
        let dictionary = dictionary
            .as_dictionary()
            .context(error::NotDictionarySnafu)?;

        for (key, value) in dictionary.records.iter() {
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
