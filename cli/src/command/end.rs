use {
    super::{CommandContext, RunCommand},
    crate::CliResult,
    solana_pubkey::Pubkey,
    tracing::info,
};

pub struct EndCommand {
    market: Pubkey,
    winner: u8,
}

impl EndCommand {
    pub fn new(market: Pubkey, winner: u8) -> Self {
        Self { market, winner }
    }
}

impl RunCommand for EndCommand {
    fn run(&self, _context: CommandContext) -> CliResult<()> {
        info!("Ending prediction market...");
        info!("Market: {}", self.market);
        info!("Winner: {}", self.winner);
        info!("This command is not yet implemented");
        Ok(())
    }
}
