use {
    super::{CommandContext, RunCommand},
    crate::{read_prediction_market_account, CliResult, WSOL},
    solana_message::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::Transaction,
    tracing::{error, info},
};
const CLAIM_INSTRUCTION: u8 = 3;
pub struct ClaimCommand {
    market: Pubkey,
}

impl ClaimCommand {
    pub fn new(market: Pubkey) -> Self {
        Self { market }
    }
}

impl RunCommand for ClaimCommand {
    fn run(&self, context: CommandContext) -> CliResult<()> {
        info!("Claiming winnings from prediction market...");
        info!("Market: {}", self.market);
        let market_data = context
            .client
            .get_account_data(&self.market)
            .map_err(|err| {
                error!("Failed to get account data: {}", err);
                err
            })?;

        let market = read_prediction_market_account(&market_data);
        let mint_pubkey = if market.winner == 1 {
            market.gamble_token_a_mint
        } else {
            market.gamble_token_b_mint
        };

        let ix_create_idempotent =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &context.keypair.pubkey(),
                &context.keypair.pubkey(),
                &Pubkey::new_from_array(mint_pubkey),
                &crate::TOKEN_PROGRAM_2022_ID,
            );

        let ix_data = [CLAIM_INSTRUCTION];
        let ix_accounts = self.get_accounts_metadata(
            &context.keypair.pubkey(),
            &self.market,
            &Pubkey::new_from_array(mint_pubkey),
        );
        let ix_claim = Instruction {
            program_id: crate::PROGRAM_ID,
            accounts: ix_accounts,
            data: ix_data.to_vec(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[ix_create_idempotent, ix_claim],
            Some(&context.keypair.pubkey()),
            &[&context.keypair],
            context.client.get_latest_blockhash().map_err(|e| {
                error!("Failed to compile message: {}", e);
                e
            })?,
        );

        context
            .client
            .send_and_confirm_transaction(&tx)
            .map_err(|e| {
                error!("Failed to send transaction: {}", e);
                e
            })?;

        info!("This command is not yet implemented");
        Ok(())
    }
}

impl ClaimCommand {
    fn get_accounts_metadata(
        &self,
        signer_pubkey: &Pubkey,
        pool_id: &Pubkey,
        mint_account: &Pubkey,
    ) -> Vec<AccountMeta> {
        let user_token_account = spl_associated_token_account::get_associated_token_address(
            &mint_account,
            &signer_pubkey,
        );
        let pool_sol_vault =
            spl_associated_token_account::get_associated_token_address(&pool_id, &WSOL);

        vec![
            AccountMeta::new(*signer_pubkey, true),
            AccountMeta::new(user_token_account, false),
            AccountMeta::new(*mint_account, false),
            AccountMeta::new(pool_sol_vault, false),
            AccountMeta::new_readonly(*pool_id, false),
        ]
    }
}
