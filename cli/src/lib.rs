mod command;

pub use command::*;
use hexis_prediction_market_interface::Prediction;
use solana_pubkey::Pubkey;
use {
    clap::{Parser, Subcommand},
    solana_client::rpc_client::RpcClient,
    solana_keypair::read_keypair_file,
    std::time::Duration,
};

const DEVNET: &str = "https://api.devnet.solana.com";
const MAINNET_BETA: &str = "https://api.mainnet-beta.solana.com";
const LOCALHOST: &str = "http://localhost:8899";
const WSOL: Pubkey = Pubkey::from_str_const("So11111111111111111111111111111111111111112");
const FEE_WALLET: Pubkey = Pubkey::from_str_const("jTGZDz9DATMcQ4fT4MKiABXYHgCF62UTAoj44PYGjQQ");

const TOKEN_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

const TOKEN_PROGRAM_2022_ID: Pubkey =
    Pubkey::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = spl_associated_token_account::ID;

const PROGRAM_ID: Pubkey = Pubkey::from_str_const("6w3daRgCgWgbvkTCXgzP5X3qYXYABpiiFWgLU6HfeJPw"); // Placeholder

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    name = "Prediction Market CLI",
    about = "CLI for managing prediction markets"
)]
pub struct Args {
    /// RPC endpoint URL or preset (mainnet-beta, devnet, localhost)
    #[arg(short = 'u', long)]
    pub url: String,

    /// Path to the keypair file
    #[arg(short)]
    pub keypair: String,

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
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("keypair error: {0}")]
    Keypair(#[from] solana_signer::SignerError),

    #[error("RPC client error: {0}")]
    RpcClient(#[from] solana_client::client_error::ClientError),

    #[error("command execution error: {0}")]
    CommandExecution(String),
}

pub type CliResult<T> = Result<T, CliError>;

pub fn run(args: Args) -> CliResult<()> {
    let url = match args.url.as_str() {
        "mainnet-beta" | "mainnet" | "m" => MAINNET_BETA,
        "devnet" | "d" => DEVNET,
        "localhost" | "l" => LOCALHOST,
        custom => custom,
    };

    let client = RpcClient::new_with_timeout(url.to_string(), Duration::from_secs(90));

    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| CliError::CommandExecution(format!("Failed to load keypair: {}", e)))?;

    let context = CommandContext { keypair, client };

    match args.command {
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

fn read_prediction_market_account(account_data: &[u8]) -> Prediction {
    Prediction {
        creator: account_data[0..32]
            .try_into()
            .expect("Failed to read creator pubkey"),
        gamble_token_a_mint: account_data[32..64]
            .try_into()
            .expect("Failed to read gamble token A mint"),
        gamble_token_b_mint: account_data[64..96]
            .try_into()
            .expect("Failed to read gamble token B mint"),
        total_token_a: u64::from_le_bytes(
            account_data[96..104]
                .try_into()
                .expect("Failed to read total_amount"),
        ),
        total_token_b: u64::from_le_bytes(
            account_data[104..112]
                .try_into()
                .expect("Failed to read total_token_b"),
        ),
        winner: u8::from_le_bytes(
            account_data[112..113]
                .try_into()
                .expect("Failed to read winner"),
        ),
        padding: account_data[113..120].try_into().expect("Missing padding"),
    }
}
