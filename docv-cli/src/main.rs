mod cli;

fn main() -> Result<(), Box<docv_ui::Error>> {
    let cli = cli::parse();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::ERROR)
        .init();

    docv_ui::run(cli.filename).map_err(|err| err.into())
}
