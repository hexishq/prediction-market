use {
    super::{CommandContext, RunCommand},
    crate::{
        read_prediction_market_account, CliResult, FEE_WALLET, PROGRAM_ID, TOKEN_PROGRAM_2022_ID,
        TOKEN_PROGRAM_ID, WSOL,
    },
    solana_client::rpc_config::UiTransactionEncoding,
    solana_message::{v0::Message, AccountMeta, Instruction, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    tracing::{error, info},
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
    fn run(&self, context: CommandContext) -> CliResult<()> {
        info!("Placing bet on prediction market...");

        let gambler_account = context.keypair.pubkey();

        let market_account = context
            .client
            .get_account(&self.market)
            .expect("Failed to fetch prediction account");

        let prediction = read_prediction_market_account(&market_account.data);

        let (prediction_account, _bump) =
            Pubkey::find_program_address(&[b"prediction", &prediction.creator], &PROGRAM_ID);

        let prediction_sol_vault =
            spl_associated_token_account::get_associated_token_address(&self.market, &WSOL);

        let mint = if self.option == 1 {
            prediction.gamble_token_a_mint
        } else if self.option == 2 {
            prediction.gamble_token_b_mint
        } else {
            panic!("Invalid option");
        };

        let token_mint = Pubkey::new_from_array(mint);

        let user_sol_account = spl_associated_token_account::get_associated_token_address(
            &context.keypair.pubkey(),
            &WSOL,
        );

        let user_token_account =
            spl_associated_token_account::get_associated_token_address_with_program_id(
                &context.keypair.pubkey(),
                &token_mint,
                &TOKEN_PROGRAM_2022_ID,
            );

        // Just to ensure easier testing
        let create_user_token_account_ix =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &context.keypair.pubkey(),
                &context.keypair.pubkey(),
                &token_mint,
                &TOKEN_PROGRAM_2022_ID,
            );

        let create_user_sol_account_ix =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &context.keypair.pubkey(),
                &context.keypair.pubkey(),
                &WSOL,
                &TOKEN_PROGRAM_ID,
            );

        let creator_sol_account = spl_associated_token_account::get_associated_token_address(
            &Pubkey::new_from_array(prediction.creator),
            &WSOL,
        );

        let protocol_fee_account =
            spl_associated_token_account::get_associated_token_address(&FEE_WALLET, &WSOL);

        let create_protocol_fee_account_ix =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &context.keypair.pubkey(),
                &protocol_fee_account,
                &WSOL,
                &TOKEN_PROGRAM_ID,
            );

        let accounts = vec![
            AccountMeta::new(gambler_account, true),
            AccountMeta::new(prediction_account, false),
            AccountMeta::new(prediction_sol_vault, false),
            AccountMeta::new(user_sol_account, false),
            AccountMeta::new(user_token_account, false),
            AccountMeta::new(token_mint, false),
            AccountMeta::new(creator_sol_account, false),
            AccountMeta::new(protocol_fee_account, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM_2022_ID, false),
        ];

        // Discriminator
        let mut instruction_data = vec![1];
        instruction_data.push(self.option);
        instruction_data.extend_from_slice(&self.amount.to_le_bytes());

        let place_bet_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts,
            // disc (u8), option(u8), amount (u64)
            data: instruction_data,
        };

        let transaction = VersionedTransaction::try_new(
            VersionedMessage::V0(
                Message::try_compile(
                    &context.keypair.pubkey(),
                    &[
                        create_protocol_fee_account_ix,
                        create_user_token_account_ix,
                        create_user_sol_account_ix,
                        place_bet_ix,
                    ],
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
                "Successfully placed {} bet on prediction {}",
                self.amount, self.market
            ),
            Err(e) => error!("Failed to place bet on {}, error: {}", self.market, e),
        }

        Ok(())
    }
}
