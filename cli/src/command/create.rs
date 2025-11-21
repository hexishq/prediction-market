use {
    super::{CommandContext, RunCommand},
    crate::CliResult,
    tracing::info,
};

pub struct CreateCommand;

impl CreateCommand {
    pub fn new() -> Self {
        Self
    }
}

impl RunCommand for CreateCommand {
    fn run(&self, _context: CommandContext) -> CliResult<()> {
        info!("Creating prediction market...");
        info!("This command is not yet implemented");
        Ok(())
    }
}
