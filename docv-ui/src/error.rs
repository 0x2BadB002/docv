use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)), context(suffix(false)))]
pub enum Error {
    #[snafu(display("Error reading PDF document"))]
    Pdf { source: docv_pdf::Error },

    #[snafu(display("Error within Iced"))]
    Iced { source: iced::Error },

    #[snafu(display("Error parsing command"))]
    Command { source: crate::app::cmdline::Error },

    #[snafu(display("{}", source.to_string()))]
    Document { source: crate::app::document::Error },
}

impl From<crate::app::document::Error> for crate::Error {
    fn from(value: crate::app::document::Error) -> Self {
        Error::Document { source: value }.into()
    }
}

impl From<crate::app::cmdline::Error> for crate::Error {
    fn from(value: crate::app::cmdline::Error) -> Self {
        Error::Command { source: value }.into()
    }
}
