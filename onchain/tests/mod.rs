use litesvm::LiteSVM;
use pinocchio::account_info;
use solana_account::Account;
use solana_gamble_onchain::BetInstruction;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::{Keypair, Signer};
use solana_pubkey::Pubkey;
use solana_transaction::Transaction;

struct TestSetup {
    pub lite_svm: LiteSVM,
    pub our_program_id: Pubkey,
}

fn setup() -> TestSetup {
    const DEFAULT_PROGRAMS: [Pubkey; 0] = [];
    const DEFAULT_PROGRAMS_PATH: &str = "programs/";
    const OUR_PROGRAM_ID: Pubkey =
        Pubkey::from_str_const("AzYYykSreF7MTjnGa62bVhuWXqWef5DYK9yz6kdHLCLw");
    const OUR_PROGRAM_PATH: &str = "./target/deploy/solana_gamble_onchain.so";

    let mut lite_svm = LiteSVM::new().with_default_programs();

    lite_svm
        .add_program_from_file(OUR_PROGRAM_ID, OUR_PROGRAM_PATH)
        .expect("Failed to load our program!");

    // Load additional programs if any
    for program in DEFAULT_PROGRAMS.iter() {
        if let Err(e) = lite_svm
            .add_program_from_file(program, format!("{}/{}.so", DEFAULT_PROGRAMS_PATH, program))
        {
            eprintln!("Failed to load program {}: {}", program, e);
        }
    }

    TestSetup {
        lite_svm,
        our_program_id: OUR_PROGRAM_ID,
    }
}

#[test]
fn test_create_bet() {
    let mut setup = setup();

    let creator_account = Keypair::new();

    setup
        .lite_svm
        .set_account(creator_account.pubkey(), Account::default())
        .expect("Airdrop to creator account failed");

    let bet_account = Keypair::new();

    let mut instruction_data = vec![];

    for byte in 1000_u64.to_le_bytes() {
        instruction_data.push(byte);
    }

    let accounts = vec![
        AccountMeta::new(creator_account.pubkey(), true),
        AccountMeta::new(bet_account.pubkey(), false),
    ];

    let instruction = Instruction {
        program_id: setup.our_program_id,
        accounts,
        data: instruction_data,
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&creator_account.pubkey()),
        &[&creator_account, &bet_account],
        setup.lite_svm.latest_blockhash(),
    );

    let result = setup
        .lite_svm
        .send_transaction(transaction)
        .expect("Transaction processing failed");

    println!("Transaction result: {}", result.pretty_logs());
}
