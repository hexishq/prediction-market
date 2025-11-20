// use litesvm::LiteSVM;
// use solana_account::Account;
// use solana_instruction::{AccountMeta, Instruction};
// use solana_keypair::{Keypair, Signer};
// use solana_pubkey::Pubkey;
// use solana_transaction::Transaction;

// struct TestSetup {
//     pub lite_svm: LiteSVM,
//     pub our_program_id: Pubkey,
// }

// fn create_mock_token_account_data(mint: Pubkey, owner: Pubkey, amount: u64) -> Vec<u8> {
//     let mut data = vec![0u8; 165];

//     data[0..32].copy_from_slice(mint.as_ref());

//     data[32..64].copy_from_slice(owner.as_ref());

//     data[64..72].copy_from_slice(&amount.to_le_bytes());

//     data[108] = 1;

//     data
// }

// #[test]
// fn test_claim_success() {
//     let mut setup = setup();

//     let user = Keypair::new();
//     let user_token_account = Keypair::new();
//     let bet_account = Keypair::new();
//     let vault_wsol = Keypair::new();
//     let vault_token_a = Keypair::new();
//     let vault_token_b = Keypair::new();

//     let mint_a = Pubkey::new_unique();
//     let mint_b = Pubkey::new_unique();

//     let bet_struct = crate::Bet {
//         creator: Pubkey::new_unique(),
//         gamble_token_a_mint: mint_a,
//         gamble_token_b_mint: mint_b,
//         gamble_vault_a: vault_token_a.pubkey(),
//         gamble_vault_b: vault_token_b.pubkey(),
//         total_amount: 1000,
//         winner: 1,
//         padding: [0; 7],
//     };

//     let mut bet_data = vec![0u8; std::mem::size_of::<crate::Bet>()];
//     let bet_bytes = bytemuck::bytes_of(&bet_struct);
//     bet_data.copy_from_slice(bet_bytes);

//     setup
//         .lite_svm
//         .set_account(
//             bet_account.pubkey(),
//             Account {
//                 lamports: 1_000_000,
//                 data: bet_data,
//                 owner: setup.our_program_id,
//                 ..Default::default()
//             },
//         )
//         .unwrap();

//     let user_winning_amount = 500;
//     let user_token_data =
//         create_mock_token_account_data(mint_a, user.pubkey(), user_winning_amount);

//     setup
//         .lite_svm
//         .set_account(
//             user_token_account.pubkey(),
//             Account {
//                 lamports: 1_000_000,
//                 data: user_token_data,
//                 owner: Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
//                 ..Default::default()
//             },
//         )
//         .unwrap();

//     let vault_a_data = create_mock_token_account_data(mint_a, bet_account.pubkey(), 1000);
//     setup
//         .lite_svm
//         .set_account(
//             vault_token_a.pubkey(),
//             Account {
//                 lamports: 1_000_000,
//                 data: vault_a_data,
//                 owner: Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
//                 ..Default::default()
//             },
//         )
//         .unwrap();

//     setup
//         .lite_svm
//         .set_account(
//             vault_wsol.pubkey(),
//             Account {
//                 lamports: 10_000_000_000,
//                 data: vec![],
//                 owner: solana_program::system_program::id(),
//                 ..Default::default()
//             },
//         )
//         .unwrap();

//     setup
//         .lite_svm
//         .airdrop(&user.pubkey(), 1_000_000_000)
//         .unwrap();

//     let instruction_data = vec![3];

//     let accounts = vec![
//         AccountMeta::new(user.pubkey(), true),
//         AccountMeta::new(user_token_account.pubkey(), false),
//         AccountMeta::new_readonly(bet_account.pubkey(), false),
//         AccountMeta::new(vault_wsol.pubkey(), false),
//         AccountMeta::new(vault_token_a.pubkey(), false),
//         AccountMeta::new(vault_token_b.pubkey(), false),
//         AccountMeta::new_readonly(solana_program::system_program::id(), false),
//     ];

//     let instruction = Instruction {
//         program_id: setup.our_program_id,
//         accounts,
//         data: instruction_data,
//     };

//     let transaction = Transaction::new_signed_with_payer(
//         &[instruction],
//         Some(&user.pubkey()),
//         &[&user],
//         setup.lite_svm.latest_blockhash(),
//     );

//     let result = setup.lite_svm.simulate_transaction(transaction);

//     match result {
//         Ok(res) => println!("Claim bem sucedido! Logs: {:?}", res.meta.log_messages),
//         Err(e) => panic!("Claim falhou: {}", e.err),
//     }
// }

// #[test]
// fn test_claim_fail_wrong_token() {
//     let mut setup = setup();

//     let user = Keypair::new();
//     let user_token_account = Keypair::new();
//     let bet_account = Keypair::new();

//     let mint_a = Pubkey::new_unique();
//     let mint_b = Pubkey::new_unique();

//     let bet_struct = crate::Bet {
//         creator: Pubkey::new_unique(),
//         gamble_token_a_mint: mint_a,
//         gamble_token_b_mint: mint_b,
//         gamble_vault_a: Pubkey::new_unique(),
//         gamble_vault_b: Pubkey::new_unique(),
//         total_amount: 1000,
//         winner: 1,
//         padding: [0; 7],
//     };

//     let mut bet_data = vec![0u8; std::mem::size_of::<crate::Bet>()];
//     bet_data.copy_from_slice(bytemuck::bytes_of(&bet_struct));
//     setup
//         .lite_svm
//         .set_account(
//             bet_account.pubkey(),
//             Account {
//                 lamports: 1_000_000,
//                 data: bet_data,
//                 owner: setup.our_program_id,
//                 ..Default::default()
//             },
//         )
//         .unwrap();

//     let user_token_data = create_mock_token_account_data(mint_b, user.pubkey(), 500);

//     setup
//         .lite_svm
//         .set_account(
//             user_token_account.pubkey(),
//             Account {
//                 lamports: 1_000_000,
//                 data: user_token_data,
//                 owner: Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
//                 ..Default::default()
//             },
//         )
//         .unwrap();

//     let instruction_data = vec![3];

//     let accounts = vec![
//         AccountMeta::new(user.pubkey(), true),
//         AccountMeta::new(user_token_account.pubkey(), false),
//         AccountMeta::new_readonly(bet_account.pubkey(), false),
//         AccountMeta::new(vault_wsol.pubkey(), false),
//         AccountMeta::new(vault_token_a.pubkey(), false),
//         AccountMeta::new(vault_token_b.pubkey(), false),
//         AccountMeta::new_readonly(solana_program::system_program::id(), false),
//     ];

//     let instruction = Instruction {
//         program_id: setup.our_program_id,
//         accounts,
//         data: instruction_data,
//     };

//     let transaction = Transaction::new_signed_with_payer(
//         &[instruction],
//         Some(&user.pubkey()),
//         &[&user],
//         setup.lite_svm.latest_blockhash(),
//     );

//     let result = setup.lite_svm.simulate_transaction(transaction);
//     assert!(result.is_err());
// }

// fn setup() -> TestSetup {
//     const DEFAULT_PROGRAMS: [Pubkey; 0] = [];
//     const DEFAULT_PROGRAMS_PATH: &str = "programs/";
//     const OUR_PROGRAM_ID: Pubkey =
//         Pubkey::from_str_const("EFPnDWAebC2S6CdqWC9gj9vcxHv3Hiba6zQfv2LxZKfR");
//     const OUR_PROGRAM_PATH: &str = "../target/deploy/solana_gamble_onchain.so";

//     let mut lite_svm = LiteSVM::new().with_default_programs().with_sysvars();

//     lite_svm
//         .add_program_from_file(OUR_PROGRAM_ID, OUR_PROGRAM_PATH)
//         .expect("Failed to load our program!");

//     for program in DEFAULT_PROGRAMS.iter() {
//         if let Err(e) = lite_svm
//             .add_program_from_file(program, format!("{}/{}.so", DEFAULT_PROGRAMS_PATH, program))
//         {
//             eprintln!("Failed to load program {}: {}", program, e);
//         }
//     }

//     TestSetup {
//         lite_svm,
//         our_program_id: OUR_PROGRAM_ID,
//     }
// }

// #[test]
// fn test_create_bet() {
//     let mut setup = setup();

//     let creator_account = Keypair::new();

//     setup
//         .lite_svm
//         .set_account(creator_account.pubkey(), Account::default())
//         .expect("Failed to set creator account");

//     setup
//         .lite_svm
//         .airdrop(&creator_account.pubkey(), 10_u64.pow(9))
//         .expect("Failed to airdrop creator account  ");

//     let bet_account = Keypair::new();

//     let mut instruction_data = vec![0];

//     for byte in 1000_u64.to_le_bytes() {
//         instruction_data.push(byte);
//     }

//     let accounts = vec![
//         AccountMeta::new(creator_account.pubkey(), true),
//         AccountMeta::new(bet_account.pubkey(), false),
//     ];

//     let instruction = Instruction {
//         program_id: setup.our_program_id,
//         accounts,
//         data: instruction_data,
//     };

//     let transaction = Transaction::new_signed_with_payer(
//         &[instruction],
//         Some(&creator_account.pubkey()),
//         &[&creator_account],
//         setup.lite_svm.latest_blockhash(),
//     );

//     match setup.lite_svm.simulate_transaction(transaction) {
//         Ok(res) => println!("Transaction succeeded with result: {:?}", res),
//         Err(e) => panic!("Transaction failed: {}", e.err),
//     }
// }
