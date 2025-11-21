use {
    clap::Parser,
    prediction_market_cli::{run, Args, CliError},
    solana_cli_config::{Config, CONFIG_FILE},
    std::sync::Arc,
};

fn main() -> Result<(), CliError> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    let args = Args::parse();

    let config_file = CONFIG_FILE.as_ref().ok_or(CliError::ConfigFilePathError)?;

    let mut config = Config::load(config_file)?;

    if let Some(custom_json_rpc_url) = args.url {
        config.json_rpc_url = custom_json_rpc_url;
    }

    if let Some(custom_keypair_path) = args.keypair {
        config.keypair_path = custom_keypair_path;
    }

    config.save(config_file)?;

    let config = Arc::new(config);

    run(config, args.command)?;

    Ok(())
}
