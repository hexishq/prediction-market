use {
    super::{CommandContext, RunCommand},
    crate::{
        CliResult, ASSOCIATED_TOKEN_PROGRAM_ID, FEE_WALLET, PROGRAM_ID, TOKEN_PROGRAM_2022_ID,
        TOKEN_PROGRAM_ID, WSOL,
    },
    solana_client::rpc_config::UiTransactionEncoding,
    solana_keypair::Keypair,
    solana_message::{v0::Message, AccountMeta, Instruction, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    tracing::{error, info},
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

        let (prediction_account, bump) = Pubkey::find_program_address(
            &[b"prediction", &signer.pubkey().to_bytes()],
            &PROGRAM_ID,
        );

        let prediction_sol_vault =
            spl_associated_token_account::get_associated_token_address(&prediction_account, &WSOL);

        let mint_a_account = Keypair::new();

        let mint_b_account = Keypair::new();

        let accounts = vec![
            AccountMeta::new(signer.pubkey(), true),
            // Prediction market account (to be created)
            AccountMeta::new(prediction_account, false),
            // SOL vault account (associated token account)
            AccountMeta::new(prediction_sol_vault, false),
            // Mint A account (keypair-based)
            AccountMeta::new(mint_a_account.pubkey(), true),
            // Mint B account (keypair-based)
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

        let mut instruction_data = vec![0];
        instruction_data.extend_from_slice(&bump.to_le_bytes());

        let create_prediction_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts,
            data: instruction_data,
        };

        // Adding it here to ensure the creator's SOL ATA exists (easier to test)
        let create_creator_sol_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &context.keypair.pubkey(),
                &context.keypair.pubkey(),
                &WSOL,
                &TOKEN_PROGRAM_ID,
            );

        // This isn't needed on production, but it's useful for testing
        let protocol_fee_sol_ata_ix =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &context.keypair.pubkey(),
                &FEE_WALLET,
                &WSOL,
                &TOKEN_PROGRAM_ID,
            );

        let transaction = VersionedTransaction::try_new(
            VersionedMessage::V0(
                Message::try_compile(
                    &signer.pubkey(),
                    &[
                        create_creator_sol_ata_ix,
                        protocol_fee_sol_ata_ix,
                        create_prediction_ix,
                    ],
                    &[],
                    context
                        .client
                        .get_latest_blockhash()
                        .expect("Failed to fetch latest blockhash"),
                )
                .expect("Failed to build VersionedMessage"),
            ),
            &[signer, &mint_a_account, &mint_b_account],
        )
        .expect("Failed to build versioned transaction");
        match context.client.send_transaction_with_config(
            &transaction,
            solana_client::rpc_config::RpcSendTransactionConfig {
                encoding: Some(UiTransactionEncoding::Base64),
                ..Default::default()
            },
        ) {
            Ok(_) => info!("Prediction {} successfully created!", prediction_account),
            Err(e) => error!(
                "Prediction creation failed for {}, error: {}",
                prediction_account, e
            ),
        }

        Ok(())
    }
}
