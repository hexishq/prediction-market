mod command;

pub use command::*;
use {
    clap::{Parser, Subcommand},
    solana_cli_config::Config,
    solana_client::rpc_client::RpcClient,
    solana_commitment_config::CommitmentConfig,
    solana_keypair::Keypair,
    solana_pubkey::Pubkey,
    std::{str::FromStr, sync::Arc, time::Duration},
};

const DEVNET: &str = "https://api.devnet.solana.com";
const MAINNET_BETA: &str = "https://api.mainnet-beta.solana.com";
const LOCALHOST: &str = "http://localhost:8899";

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    name = "Prediction Market CLI",
    about = "CLI for managing prediction markets"
)]
pub struct Args {
    /// RPC endpoint URL or preset (mainnet-beta, devnet, localhost)
    #[arg(short = 'u', long, global = true)]
    pub url: Option<String>,

    /// Path to the keypair file
    #[arg(short = 'k', long, global = true)]
    pub keypair: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create a new prediction market
    Create,

    /// Place a bet on a prediction market
    PlaceBet {
        /// The prediction market address
        #[arg(long)]
        market: Pubkey,

        /// Which option to bet on (1 or 2)
        #[arg(long)]
        option: u8,

        /// Amount to bet in lamports
        #[arg(long)]
        amount: u64,
    },

    /// End a prediction market and set the winner
    End {
        /// The prediction market address
        #[arg(long)]
        market: Pubkey,

        /// Which option won (1 or 2)
        #[arg(long)]
        winner: u8,
    },

    /// Claim winnings from a prediction market
    Claim {
        /// The prediction market address
        #[arg(long)]
        market: Pubkey,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum CliError {
    #[error("unable to get config file path")]
    ConfigFilePathError,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("keypair error: {0}")]
    Keypair(#[from] solana_signer::SignerError),

    #[error("RPC client error: {0}")]
    RpcClient(#[from] solana_client::client_error::ClientError),

    #[error("commitment config error: {0}")]
    CommitmentConfig(#[from] solana_commitment_config::ParseCommitmentLevelError),

    #[error("command execution error: {0}")]
    CommandExecution(String),
}

pub type CliResult<T> = Result<T, CliError>;

pub fn run(config: Arc<Config>, command: Command) -> CliResult<()> {
    let url = resolve_cluster_url(&config.json_rpc_url);

    let client = RpcClient::new_with_timeout_and_commitment(
        url,
        Duration::from_secs(90),
        CommitmentConfig::from_str(&config.commitment)?,
    );

    let keypair = load_keypair(&config.keypair_path)?;

    let context = CommandContext { keypair, client };

    match command {
        Command::Create => {
            create::CreateCommand::new().run(context)?;
        }
        Command::PlaceBet {
            market,
            option,
            amount,
        } => {
            place_bet::PlaceBetCommand::new(market, option, amount).run(context)?;
        }
        Command::End { market, winner } => {
            end::EndCommand::new(market, winner).run(context)?;
        }
        Command::Claim { market } => {
            claim::ClaimCommand::new(market).run(context)?;
        }
    }

    Ok(())
}

fn resolve_cluster_url(url: &str) -> String {
    match url {
        "mainnet-beta" | "mainnet" | "m" => MAINNET_BETA.to_string(),
        "devnet" | "d" => DEVNET.to_string(),
        "localhost" | "l" => LOCALHOST.to_string(),
        custom => custom.to_string(),
    }
}

fn load_keypair(keypair_path: &str) -> CliResult<Keypair> {
    solana_keypair::read_keypair_file(keypair_path)
        .map_err(|e| CliError::CommandExecution(format!("Failed to load keypair: {}", e)))
}
