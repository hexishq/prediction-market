use {
    super::{CommandContext, RunCommand},
    crate::{
        CliResult, ASSOCIATED_TOKEN_PROGRAM_ID, PROGRAM_ID, TOKEN_PROGRAM_2022_ID,
        TOKEN_PROGRAM_ID, WSOL,
    },
    solana_client::rpc_config::UiTransactionEncoding,
    solana_keypair::Keypair,
    solana_message::{v0::Message, AccountMeta, Instruction, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    tracing::info,
};

pub struct CreateCommand;

impl CreateCommand {
    pub fn new() -> Self {
        Self
    }
}

impl RunCommand for CreateCommand {
    fn run(&self, context: CommandContext) -> CliResult<()> {
        info!("Creating prediction market...");

        let signer = &context.keypair;

        let prediction_account = Keypair::new();

        let prediction_sol_vault = spl_associated_token_account::get_associated_token_address(
            &prediction_account.pubkey(),
            &WSOL,
        );

        let mint_a_account = Keypair::new();

        let mint_b_account = Keypair::new();

        println!("Mint A: {}", mint_a_account.pubkey());
        println!("Mint B: {}", mint_b_account.pubkey());

        let accounts = vec![
            AccountMeta::new(signer.pubkey(), true),
            // Prediction market account (to be created)
            AccountMeta::new(prediction_account.pubkey(), true),
            // SOL vault account (associated token account)
            AccountMeta::new(prediction_sol_vault, false),
            // Mint A account (PDA)
            AccountMeta::new(mint_a_account.pubkey(), true),
            // Mint B account (PDA)
            AccountMeta::new(mint_b_account.pubkey(), true),
            // Wsol Mint
            AccountMeta::new_readonly(WSOL, false),
            // System program
            AccountMeta::new_readonly(Pubkey::default(), false),
            // SPL Token program
            AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
            // SPL Token 2022 program
            AccountMeta::new_readonly(TOKEN_PROGRAM_2022_ID, false),
            // Associated token program
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM_ID, false),
        ];

        let create_prediction_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts,
            data: vec![0],
        };

        let transaction = VersionedTransaction::try_new(
            VersionedMessage::V0(
                Message::try_compile(
                    &signer.pubkey(),
                    &[create_prediction_ix],
                    &[],
                    context
                        .client
                        .get_latest_blockhash()
                        .expect("Failed to fetch latest blockhash"),
                )
                .expect("Failed to build VersionedMessage"),
            ),
            &[
                signer,
                &prediction_account,
                &mint_a_account,
                &mint_b_account,
            ],
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
                "Prediction {} successfully created!",
                prediction_account.pubkey()
            ),
            Err(e) => tracing::error!(
                "Prediction creation failed for {}, error: {}",
                prediction_account.pubkey(),
                e
            ),
        }

        Ok(())
    }
}
