pub use error::{Error, Result};

mod app;
mod cli;
mod error;

fn main() -> Result<()> {
    let cli = cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::ERROR)
        .init();

    app::run(cli.filename)
}
