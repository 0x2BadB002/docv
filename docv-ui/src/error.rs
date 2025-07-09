pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Pdf(#[from] docv_pdf::Error),
    #[error(transparent)]
    Iced(#[from] iced::Error),

    #[error("Failed to parse command: {0}")]
    ParserError(String),
}
