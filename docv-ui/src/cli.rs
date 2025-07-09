use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    pub filename: Option<PathBuf>,
}

pub fn parse() -> Cli {
    Cli::parse()
}
