use snafu::Snafu;

pub type Result<T> = std::result::Result<T, Box<Error>>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)), context(suffix(false)))]
pub enum Error {
    #[snafu(display("Error reading PDF document"))]
    Pdf { source: docv_pdf::Error },

    #[snafu(display("Error within Iced"))]
    Iced { source: iced::Error },

    #[snafu(display("{}", source.to_string()))]
    Command { source: crate::app::cmdline::Error },

    #[snafu(display("{}", source.to_string()))]
    Document { source: crate::app::document::Error },

    #[snafu(display("No file specified"))]
    NoFile,

    #[snafu(display("Modal dialog error"))]
    ModalDialog { source: ashpd::Error },

    #[snafu(display("Failed to convert path"))]
    Path,
}
