use {
    super::{CommandContext, RunCommand},
    crate::{
        read_prediction_market_account, CliResult, TOKEN_PROGRAM_2022_ID, TOKEN_PROGRAM_ID, WSOL,
    },
    solana_client::rpc_config::UiTransactionEncoding,
    solana_message::{AccountMeta, Instruction},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::Transaction,
    tracing::{error, info},
};

const CLAIM_INSTRUCTION_DISCRIMINATOR: u8 = 3;

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

        let market_data = context
            .client
            .get_account_data(&self.market)
            .map_err(|err| {
                error!("Failed to get account data: {}", err);
                err
            })?;

        let prediction = read_prediction_market_account(&market_data);

        let token_mint = if prediction.winner == 1 {
            prediction.gamble_token_a_mint
        } else {
            prediction.gamble_token_b_mint
        };

        let create_idempotent_ix =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &context.keypair.pubkey(),
                &context.keypair.pubkey(),
                &Pubkey::new_from_array(token_mint),
                &crate::TOKEN_PROGRAM_2022_ID,
            );

        let instruction_data = [CLAIM_INSTRUCTION_DISCRIMINATOR];

        let accounts = self.get_accounts_metadata(
            &context.keypair.pubkey(),
            &self.market,
            &Pubkey::new_from_array(token_mint),
        );

        let claim_ix = Instruction {
            program_id: crate::PROGRAM_ID,
            accounts,
            data: instruction_data.to_vec(),
        };

        let tx = Transaction::new_signed_with_payer(
            &[create_idempotent_ix, claim_ix],
            Some(&context.keypair.pubkey()),
            &[&context.keypair],
            context.client.get_latest_blockhash().map_err(|e| {
                error!("Failed to compile message: {}", e);
                e
            })?,
        );

        match context.client.send_transaction_with_config(
            &tx,
            solana_client::rpc_config::RpcSendTransactionConfig {
                encoding: Some(UiTransactionEncoding::Base64),
                ..Default::default()
            },
        ) {
            Ok(_) => info!("Prediction {} successfully claimed!", self.market),
            Err(e) => error!(
                "Prediction claim failed for {}, error: {}",
                self.market,
                e.to_string()
            ),
        }
        Ok(())
    }
}

impl ClaimCommand {
    fn get_accounts_metadata(
        &self,
        signer_pubkey: &Pubkey,
        prediction_id: &Pubkey,
        mint_account: &Pubkey,
    ) -> Vec<AccountMeta> {
        let user_token_account =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &signer_pubkey,
                &mint_account,
                &TOKEN_PROGRAM_2022_ID,
            );

        let user_sol_account =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &signer_pubkey,
                &WSOL,
                &TOKEN_PROGRAM_ID,
            );

        let prediction_sol_vault =
            spl_associated_token_account::get_associated_token_address(&prediction_id, &WSOL);

        vec![
            AccountMeta::new(*signer_pubkey, true),
            AccountMeta::new(user_token_account, false),
            AccountMeta::new(user_sol_account, false),
            AccountMeta::new(*mint_account, false),
            AccountMeta::new(prediction_sol_vault, false),
            AccountMeta::new(*prediction_id, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_2022_ID, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        ]
    }
}
