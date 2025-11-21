pub mod claim;
pub mod create;
pub mod end;
pub mod place_bet;

use {crate::CliResult, solana_client::rpc_client::RpcClient, solana_keypair::Keypair};

pub struct CommandContext {
    pub client: RpcClient,
    pub keypair: Keypair,
}

pub trait RunCommand {
    fn run(&self, context: CommandContext) -> CliResult<()>;
}
