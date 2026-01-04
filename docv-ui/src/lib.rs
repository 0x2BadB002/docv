use snafu::Snafu;

mod app;
mod error;

pub use app::run;

#[derive(Debug, Snafu)]
#[snafu(source(from(error::Error, Box::new)))]
pub struct Error(Box<error::Error>);
type Result<T> = std::result::Result<T, Error>;
