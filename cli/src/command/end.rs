use {
    super::{CommandContext, RunCommand},
    crate::{CliResult, PROGRAM_ID},
    solana_client::rpc_config::UiTransactionEncoding,
    solana_message::{v0::Message, AccountMeta, Instruction, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    tracing::{error, info},
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
    fn run(&self, context: CommandContext) -> CliResult<()> {
        info!("Ending prediction market...");

        // Creator must be signer in order to end the market
        let creator_account = context.keypair.pubkey();

        // Discriminator, winner option
        let instruction_data = vec![2, self.winner];

        let accounts = vec![
            AccountMeta::new(creator_account, true),
            AccountMeta::new(self.market, false),
        ];

        let end_prediction_ix =
            Instruction::new_with_bytes(PROGRAM_ID, &instruction_data, accounts);

        let transaction = VersionedTransaction::try_new(
            VersionedMessage::V0(
                Message::try_compile(
                    &creator_account,
                    &[end_prediction_ix],
                    &[],
                    context
                        .client
                        .get_latest_blockhash()
                        .expect("Failed to fetch latest blockhash"),
                )
                .expect("Failed to build VersionedMessage"),
            ),
            &[context.keypair],
        )
        .expect("Failed to build versioned transaction");

        match context.client.send_transaction_with_config(
            &transaction,
            solana_client::rpc_config::RpcSendTransactionConfig {
                encoding: Some(UiTransactionEncoding::Base64),
                ..Default::default()
            },
        ) {
            Ok(_) => info!(
                "Prediction {} successfully ended, winner is {}!",
                self.market, self.winner
            ),
            Err(e) => error!("Prediction settle failed for {}, error: {}", self.market, e),
        }

        Ok(())
    }
}
