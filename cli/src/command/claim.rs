use {
    super::{CommandContext, RunCommand},
    crate::CliResult,
    solana_pubkey::Pubkey,
    tracing::info,
};

pub struct ClaimCommand {
    market: Pubkey,
}

impl ClaimCommand {
    pub fn new(market: Pubkey) -> Self {
        Self { market }
    }
}

impl RunCommand for ClaimCommand {
    fn run(&self, _context: CommandContext) -> CliResult<()> {
        info!("Claiming winnings from prediction market...");
        info!("Market: {}", self.market);
        info!("This command is not yet implemented");
        Ok(())
    }
}
