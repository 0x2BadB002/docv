use snafu::Snafu;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub enum Error {
    #[snafu(display("Error reading PDF document"))]
    Pdf { source: docv_pdf::Error },

    #[snafu(display("Error within Iced"))]
    Iced { source: iced::Error },

    #[snafu(display("Error parsing command"))]
    Command { source: crate::app::cmdline::Error },
}
