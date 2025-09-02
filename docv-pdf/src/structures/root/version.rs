use snafu::{ResultExt, Snafu};

use crate::types::Object;

#[derive(Debug, Snafu)]
pub struct Error(error::Error);
type Result<T> = std::result::Result<T, Error>;

/// Represents the version of a PDF document.
///
/// PDF versions follow a major.minor numbering scheme where:
/// - Major versions are either 1 or 2
/// - Minor versions range from 0-7 for PDF 1.x
/// - PDF 2.0 uses major version 2 with minor version 0
///
/// The version is typically found in the PDF header and determines
/// which features are available in the document.
#[derive(Debug, Default, PartialEq, Clone)]
pub enum Version {
    /// PDF Version 1.0 (1993)
    Pdf1_0,
    /// PDF Version 1.1 (1996)
    Pdf1_1,
    /// PDF Version 1.2 (1996)
    Pdf1_2,
    /// PDF Version 1.3 (2000)
    Pdf1_3,
    /// PDF Version 1.4 (2001)
    Pdf1_4,
    /// PDF Version 1.5 (2003)
    Pdf1_5,
    /// PDF Version 1.6 (2004)
    Pdf1_6,
    /// PDF Version 1.7 (2006)
    Pdf1_7,
    /// PDF Version 2.0 (2017)
    #[default]
    Pdf2_0,
}

impl Version {
    pub fn from_str(source: &str) -> Result<Self> {
        match source {
            "1.0" => Ok(Version::Pdf1_0),
            "1.1" => Ok(Version::Pdf1_1),
            "1.2" => Ok(Version::Pdf1_2),
            "1.3" => Ok(Version::Pdf1_3),
            "1.4" => Ok(Version::Pdf1_4),
            "1.5" => Ok(Version::Pdf1_5),
            "1.6" => Ok(Version::Pdf1_6),
            "1.7" => Ok(Version::Pdf1_7),
            "2.0" => Ok(Version::Pdf2_0),
            _ => Err(error::Error::UnknownVersion {
                data: source.to_string(),
            }
            .into()),
        }
    }

    pub fn from_bytes(source: &[u8]) -> Result<Self> {
        let version_str = str::from_utf8(source).with_context(|_| error::InvalidBytesSnafu {
            data: source.to_vec(),
        })?;

        Self::from_str(version_str)
    }

    pub fn from_object(object: &Object) -> Result<Self> {
        let name = object.as_name().context(error::InvalidObjectSnafu)?;

        Self::from_str(name)
    }

    pub fn as_str(&self) -> &str {
        match self {
            Version::Pdf1_0 => "1.0",
            Version::Pdf1_1 => "1.1",
            Version::Pdf1_2 => "1.2",
            Version::Pdf1_3 => "1.3",
            Version::Pdf1_4 => "1.4",
            Version::Pdf1_5 => "1.5",
            Version::Pdf1_6 => "1.6",
            Version::Pdf1_7 => "1.7",
            Version::Pdf2_0 => "2.0",
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

mod error {
    use snafu::Snafu;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub(super)))]
    pub(super) enum Error {
        #[snafu(display("Unknown version string passed: {data}"))]
        UnknownVersion { data: String },

        #[snafu(display("Invalid bytes passed: {data:?}"))]
        InvalidBytes {
            data: Vec<u8>,
            source: std::str::Utf8Error,
        },

        #[snafu(display("Invalid object passed"))]
        InvalidObject { source: crate::types::ObjectError },
    }
}
