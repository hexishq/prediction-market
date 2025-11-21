use {
    super::{CommandContext, RunCommand},
    crate::CliResult,
    solana_pubkey::Pubkey,
    tracing::info,
};

pub struct PlaceBetCommand {
    market: Pubkey,
    option: u8,
    amount: u64,
}

impl PlaceBetCommand {
    pub fn new(market: Pubkey, option: u8, amount: u64) -> Self {
        Self {
            market,
            option,
            amount,
        }
    }
}

impl RunCommand for PlaceBetCommand {
    fn run(&self, _context: CommandContext) -> CliResult<()> {
        info!("Placing bet on prediction market...");
        info!("Market: {}", self.market);
        info!("Option: {}", self.option);
        info!("Amount: {} lamports", self.amount);
        info!("This command is not yet implemented");
        Ok(())
    }
}
