use {
    clap::Parser,
    prediction_market_cli::{run, Args, CliError},
};

fn main() -> Result<(), CliError> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    let args = Args::parse();

    run(args)?;

    Ok(())
}
