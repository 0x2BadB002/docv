use snafu::{ResultExt, Snafu};

use crate::types::{Object, string::Date};

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

/// Contains the standard document information dictionary from a PDF file.
///
/// This struct holds metadata about the PDF document such as title, author,
/// creation date, and other properties defined in the PDF specification.
/// It can also store custom key-value pairs not covered by the standard fields.
#[derive(Debug, Default)]
pub struct Info {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<Date>,
    pub mod_date: Option<Date>,
    pub trapped: Trap,
    pub other: Vec<(String, String)>,
}

/// Indicates whether a PDF document has been trapped.
///
/// Trapping is a prepress technique to prevent gaps between colored areas
/// during the printing process.
#[derive(Debug, Default)]
pub enum Trap {
    True,
    False,
    #[default]
    Unknown,
}

impl Info {
    pub fn from_object(object: Object) -> Result<Self> {
        let mut result = Self::default();

        if object.is_null() {
            return Ok(result);
        }

        let dictionary = object.as_dictionary().context(error::NotDictionary)?;

        for (key, value) in dictionary.iter() {
            match key.as_str() {
                "Title" => {
                    result.title = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidField { field: key.clone() })?
                            .as_str()
                            .context(error::PdfString)?
                            .to_string(),
                    )
                }
                "Author" => {
                    result.author = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidField { field: key.clone() })?
                            .as_str()
                            .context(error::PdfString)?
                            .to_string(),
                    )
                }
                "Subject" => {
                    result.subject = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidField { field: key.clone() })?
                            .as_str()
                            .context(error::PdfString)?
                            .to_string(),
                    )
                }
                "Keywords" => {
                    result.keywords = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidField { field: key.clone() })?
                            .as_str()
                            .context(error::PdfString)?
                            .to_string(),
                    )
                }
                "Creator" => {
                    result.creator = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidField { field: key.clone() })?
                            .as_str()
                            .context(error::PdfString)?
                            .to_string(),
                    )
                }
                "Producer" => {
                    result.producer = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidField { field: key.clone() })?
                            .as_str()
                            .context(error::PdfString)?
                            .to_string(),
                    )
                }
                "CreationDate" => {
                    result.creation_date = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidField { field: key.clone() })?
                            .to_date()
                            .context(error::PdfString)?,
                    )
                }
                "ModDate" => {
                    result.mod_date = Some(
                        value
                            .as_string()
                            .with_context(|_| error::InvalidField { field: key.clone() })?
                            .to_date()
                            .context(error::PdfString)?,
                    )
                }
                "Trapped" => {
                    let value = value
                        .as_string()
                        .with_context(|_| error::InvalidField { field: key.clone() })?
                        .as_str()
                        .context(error::PdfString)?;

                    result.trapped = match value {
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
                _ => result.other.push((
                    key.to_string(),
                    value
                        .as_string()
                        .with_context(|_| error::InvalidField { field: key.clone() })?
                        .as_str()
                        .context(error::PdfString)?
                        .to_string(),
                )),
            }
        }

        Ok(result)
    }
}

impl std::fmt::Display for Trap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Trap::True => "True",
                Trap::False => "False",
                Trap::Unknown => "Unknown",
            }
        )
    }
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)), context(suffix(false)))]
    pub(super) enum Error {
        #[snafu(display("Parsed object is not dictionary"))]
        NotDictionary { source: crate::types::object::Error },

        #[snafu(display("Wrong field {field} data format"))]
        InvalidField {
            field: String,
            source: crate::types::object::Error,
        },

        #[snafu(display("Unexpected Trapping value encountered. Value = {value}"))]
        UnexpectedTrapValue { value: String },

        #[snafu(display("Error while working with pdf string"))]
        PdfString { source: crate::types::string::Error },
    }
}
