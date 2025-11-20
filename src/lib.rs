use bytemuck::{Pod, Zeroable};
use pinocchio::{
    account_info::AccountInfo,
    entrypoint, msg,
    program_error::ProgramError,
    pubkey::{find_program_address, Pubkey},
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
mod token_account;
use pinocchio_system::instructions::Transfer;
use token_account::*;

entrypoint!(process_instruction);

const TOKEN_PROGRAM_2022: Pubkey = [
    6, 221, 246, 225, 238, 117, 143, 222, 24, 66, 93, 188, 228, 108, 205, 218, 182, 26, 252, 77,
    131, 185, 13, 39, 254, 189, 249, 40, 216, 161, 139, 252,
];

// Define the data structures for the program
#[repr(C, packed)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct Bet {
    pub creator: Pubkey,             // 0-32
    pub gamble_token_a_mint: Pubkey, // 32-64
    pub gamble_token_b_mint: Pubkey, // 64 -96
    pub gamble_vault_a: Pubkey,      // 96-128
    pub gamble_vault_b: Pubkey,      // 128-160
    pub total_amount: u64,           // 160-168
    pub winner: u8,                  // 168-169
    pub padding: [u8; 7],            // 169-176
}

pub enum BetInstruction {
    CreatePrediction { amount: u64 }, // admin command
    EndPrediction { winner: u8 },     // admin command
    PlaceBet { option: u8, amount: u64 },
    Claim,
}

impl BetInstruction {
    // Unpack the instruction data
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match tag {
            0 => {
                let amount = rest
                    .get(..8)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                Self::CreatePrediction { amount }
            }
            1 => {
                let option = rest.get(0).ok_or(ProgramError::InvalidInstructionData)?;
                let amount = rest
                    .get(1..9)
                    .and_then(|slice| slice.try_into().ok())
                    .map(u64::from_le_bytes)
                    .ok_or(ProgramError::InvalidInstructionData)?;
                Self::PlaceBet {
                    option: *option,
                    amount,
                }
            }
            2 => {
                let winner = rest.get(0).ok_or(ProgramError::InvalidInstructionData)?;
                Self::EndPrediction { winner: *winner }
            }
            3 => Self::Claim,
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }
}

// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = BetInstruction::unpack(instruction_data)?;

    match instruction {
        BetInstruction::CreatePrediction { amount } => {
            msg!("Instruction: CreateBet");
            create_bet(program_id, accounts, amount)
        }
        BetInstruction::PlaceBet { option, amount } => {
            msg!("Instruction: PlaceBet");
            place_bet(program_id, accounts, option, amount)
        }
        BetInstruction::EndPrediction { winner } => {
            msg!("Instruction: SettleBet");
            settle_bet(program_id, accounts, winner)
        }
        BetInstruction::Claim => {
            msg!("Instruction: Claim");
            claim(accounts)
        }
    }
}

// Create a new bet
fn create_bet(program_id: &Pubkey, accounts: &[AccountInfo], _amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let creator_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let bet_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let vault_a_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let vault_b_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let mint_a_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let mint_b_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let system_program = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let token_program = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let (mint_a, _bump) =
        find_program_address(&[bet_account.key(), &1_u64.to_le_bytes()], program_id);
    let (mint_b, _bump) =
        find_program_address(&[bet_account.key(), &2_u64.to_le_bytes()], program_id);

    if mint_a_account.key() != &mint_a {
        msg!("Mint A account does not match the derived address");
        return Err(ProgramError::InvalidAccountData);
    }

    if mint_b_account.key() != &mint_b {
        msg!("Mint B account does not match the derived address");
        return Err(ProgramError::InvalidAccountData);
    }

    initialize_mint(9, mint_a_account, program_id, Some(program_id))?;
    initialize_mint(9, mint_b_account, program_id, Some(program_id))?;

    let rent = Rent::get()?;

    msg!("Creating bet account");
    // Create the bet account
    pinocchio_system::instructions::CreateAccount {
        from: creator_account,
        to: bet_account,
        lamports: rent.minimum_balance(std::mem::size_of::<Bet>()),
        space: std::mem::size_of::<Bet>() as u64,
        owner: program_id,
    }
    .invoke()?;

    // Create pool vaults
    pinocchio_associated_token_account::instructions::Create {
        funding_account: creator_account,
        account: vault_a_account,
        wallet: bet_account,
        mint: mint_a_account,
        system_program: system_program,
        token_program: token_program,
    }
    .invoke()?;

    pinocchio_associated_token_account::instructions::Create {
        funding_account: creator_account,
        account: vault_b_account,
        wallet: bet_account,
        mint: mint_b_account,
        system_program: system_program,
        token_program: token_program,
    }
    .invoke()?;

    let mut bet_data = bet_account.try_borrow_mut_data()?;
    let bet = bytemuck::from_bytes_mut::<Bet>(&mut bet_data);

    bet.creator = *creator_account.key();
    bet.total_amount = 0;
    bet.winner = 0;
    bet.gamble_token_a_mint = mint_a;
    bet.gamble_token_b_mint = mint_b;
    bet.gamble_vault_a = *vault_a_account.key();
    bet.gamble_vault_b = *vault_b_account.key();

    Ok(())
}

fn initialize_mint(
    decimals: u8,
    mint_account: &AccountInfo,
    mint_authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
) -> ProgramResult {
    pinocchio_token_2022::instructions::InitializeMint2 {
        mint: mint_account,
        decimals,
        mint_authority,
        freeze_authority,
        token_program: &TOKEN_PROGRAM_2022,
    }
    .invoke()
}

// Place a bet on one of the alternatives
fn place_bet(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    option: u8,
    amount: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let gambler_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let bet_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    if *bet_account.owner() != *program_id {
        msg!("Bet account not owned by the program");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut bet_data = bet_account.try_borrow_mut_data()?;
    let bet = bytemuck::from_bytes_mut::<Bet>(&mut bet_data);

    if bet.winner != 0 {
        msg!("Bet already settled");
        return Err(ProgramError::InvalidAccountData);
    }

    if option != 1 && option != 2 {
        msg!("Invalid option");
        return Err(ProgramError::InvalidInstructionData);
    }

    // Transfer lamports from gambler to bet account
    pinocchio_system::instructions::Transfer {
        from: gambler_account,
        to: bet_account,
        lamports: amount,
    }
    .invoke()?;

    bet.total_amount += amount;

    Ok(())
}

// Settle the bet and distribute the funds
fn settle_bet(program_id: &Pubkey, accounts: &[AccountInfo], winner: u8) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let creator_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let bet_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    if *bet_account.owner() != *program_id {
        msg!("Bet account not owned by the program");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut bet_data = bet_account.try_borrow_mut_data()?;
    let bet = bytemuck::from_bytes_mut::<Bet>(&mut bet_data);

    if *creator_account.key() != bet.creator {
        msg!("Only the creator can settle the bet");
        return Err(ProgramError::IllegalOwner);
    }

    if bet.winner != 0 {
        msg!("Bet already settled");
        return Err(ProgramError::InvalidAccountData);
    }

    bet.winner = winner;

    Ok(())
}

// [signer, signer_token_account, pool, vault_wsol, vault_token_a, vault_token_b]
fn claim(accounts: &[AccountInfo]) -> ProgramResult {
    let signer = accounts.get(0).ok_or(ProgramError::InvalidAccountData)?;
    let signer_token_account = accounts.get(1).ok_or(ProgramError::InvalidAccountData)?;
    let pool = accounts.get(2).ok_or(ProgramError::InvalidAccountData)?;
    let vault_wsol = accounts.get(3).ok_or(ProgramError::InvalidAccountData)?;
    let vault_token_a = accounts.get(4).ok_or(ProgramError::InvalidAccountData)?;
    let vault_token_b = accounts.get(5).ok_or(ProgramError::InvalidAccountData)?;

    let token_mint = unsafe { AtaAccessor::get_mint(signer_token_account.borrow_data_unchecked()) };

    let winner: u8 = unsafe { pool.borrow_data_unchecked()[168] };
    let (win_mint, to) = if winner == 1 {
        (
            unsafe { AtaAccessor::get_mint(vault_token_a.borrow_data_unchecked()) },
            vault_token_a,
        )
    } else if winner == 2 {
        (
            unsafe { AtaAccessor::get_mint(vault_token_b.borrow_data_unchecked()) },
            vault_token_b,
        )
    } else {
        return Err(ProgramError::InvalidAccountData);
    };

    let winner_amount = AtaAccessor::get_amount(signer_token_account.key());

    if win_mint != token_mint {
        return Err(ProgramError::InvalidAccountData);
    }

    let signer_to_pool = Transfer {
        from: signer,
        to,
        lamports: winner_amount,
    };

    let pool_to_signer = Transfer {
        from: vault_wsol,
        to: signer,
        lamports: winner_amount,
    };

    signer_to_pool.invoke();
    pool_to_signer.invoke();

    Ok(())
}

#[cfg(test)]
mod tests {
    use litesvm::LiteSVM;
    use solana_account::Account;
    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::{Keypair, Signer};
    use solana_pubkey::Pubkey;
    use solana_transaction::Transaction;

    struct TestSetup {
        pub lite_svm: LiteSVM,
        pub our_program_id: Pubkey,
    }

    fn create_mock_token_account_data(mint: Pubkey, owner: Pubkey, amount: u64) -> Vec<u8> {
        let mut data = vec![0u8; 165];

        data[0..32].copy_from_slice(mint.as_ref());

        data[32..64].copy_from_slice(owner.as_ref());

        data[64..72].copy_from_slice(&amount.to_le_bytes());

        data[108] = 1;

        data
    }

    #[test]
    fn test_claim_success() {
        let mut setup = setup();

        let user = Keypair::new();
        let user_token_account = Keypair::new();
        let bet_account = Keypair::new();
        let vault_wsol = Keypair::new();
        let vault_token_a = Keypair::new();
        let vault_token_b = Keypair::new();

        let mint_a = Pubkey::new_unique();
        let mint_b = Pubkey::new_unique();

        let bet_struct = crate::Bet {
            creator: Pubkey::new_unique(),
            gamble_token_a_mint: mint_a,
            gamble_token_b_mint: mint_b,
            gamble_vault_a: vault_token_a.pubkey(),
            gamble_vault_b: vault_token_b.pubkey(),
            total_amount: 1000,
            winner: 1,
            padding: [0; 7],
        };

        let mut bet_data = vec![0u8; std::mem::size_of::<crate::Bet>()];
        let bet_bytes = bytemuck::bytes_of(&bet_struct);
        bet_data.copy_from_slice(bet_bytes);

        setup
            .lite_svm
            .set_account(
                bet_account.pubkey(),
                Account {
                    lamports: 1_000_000,
                    data: bet_data,
                    owner: setup.our_program_id,
                    ..Default::default()
                },
            )
            .unwrap();

        let user_winning_amount = 500;
        let user_token_data =
            create_mock_token_account_data(mint_a, user.pubkey(), user_winning_amount);

        setup
            .lite_svm
            .set_account(
                user_token_account.pubkey(),
                Account {
                    lamports: 1_000_000,
                    data: user_token_data,
                    owner: Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
                    ..Default::default()
                },
            )
            .unwrap();

        let vault_a_data = create_mock_token_account_data(mint_a, bet_account.pubkey(), 1000);
        setup
            .lite_svm
            .set_account(
                vault_token_a.pubkey(),
                Account {
                    lamports: 1_000_000,
                    data: vault_a_data,
                    owner: Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
                    ..Default::default()
                },
            )
            .unwrap();

        setup
            .lite_svm
            .set_account(
                vault_wsol.pubkey(),
                Account {
                    lamports: 10_000_000_000,
                    data: vec![],
                    owner: solana_program::system_program::id(),
                    ..Default::default()
                },
            )
            .unwrap();

        setup
            .lite_svm
            .airdrop(&user.pubkey(), 1_000_000_000)
            .unwrap();

        let instruction_data = vec![3];

        let accounts = vec![
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new(user_token_account.pubkey(), false),
            AccountMeta::new_readonly(bet_account.pubkey(), false),
            AccountMeta::new(vault_wsol.pubkey(), false),
            AccountMeta::new(vault_token_a.pubkey(), false),
            AccountMeta::new(vault_token_b.pubkey(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ];

        let instruction = Instruction {
            program_id: setup.our_program_id,
            accounts,
            data: instruction_data,
        };

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&user.pubkey()),
            &[&user],
            setup.lite_svm.latest_blockhash(),
        );

        let result = setup.lite_svm.simulate_transaction(transaction);

        match result {
            Ok(res) => println!("Claim bem sucedido! Logs: {:?}", res.meta.log_messages),
            Err(e) => panic!("Claim falhou: {}", e.err),
        }
    }

    #[test]
    fn test_claim_fail_wrong_token() {
        let mut setup = setup();

        let user = Keypair::new();
        let user_token_account = Keypair::new();
        let bet_account = Keypair::new();

        let mint_a = Pubkey::new_unique();
        let mint_b = Pubkey::new_unique();

        let bet_struct = crate::Bet {
            creator: Pubkey::new_unique(),
            gamble_token_a_mint: mint_a,
            gamble_token_b_mint: mint_b,
            gamble_vault_a: Pubkey::new_unique(),
            gamble_vault_b: Pubkey::new_unique(),
            total_amount: 1000,
            winner: 1,
            padding: [0; 7],
        };

        let mut bet_data = vec![0u8; std::mem::size_of::<crate::Bet>()];
        bet_data.copy_from_slice(bytemuck::bytes_of(&bet_struct));
        setup
            .lite_svm
            .set_account(
                bet_account.pubkey(),
                Account {
                    lamports: 1_000_000,
                    data: bet_data,
                    owner: setup.our_program_id,
                    ..Default::default()
                },
            )
            .unwrap();

        let user_token_data = create_mock_token_account_data(mint_b, user.pubkey(), 500);

        setup
            .lite_svm
            .set_account(
                user_token_account.pubkey(),
                Account {
                    lamports: 1_000_000,
                    data: user_token_data,
                    owner: Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
                    ..Default::default()
                },
            )
            .unwrap();

        let instruction_data = vec![3];

        let accounts = vec![
            AccountMeta::new(user.pubkey(), true),
            AccountMeta::new(user_token_account.pubkey(), false),
            AccountMeta::new_readonly(bet_account.pubkey(), false),
            AccountMeta::new(vault_wsol.pubkey(), false),
            AccountMeta::new(vault_token_a.pubkey(), false),
            AccountMeta::new(vault_token_b.pubkey(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ];

        let instruction = Instruction {
            program_id: setup.our_program_id,
            accounts,
            data: instruction_data,
        };

        let transaction = Transaction::new_signed_with_payer(
            &[instruction],
            Some(&user.pubkey()),
            &[&user],
            setup.lite_svm.latest_blockhash(),
        );

        let result = setup.lite_svm.simulate_transaction(transaction);
        assert!(result.is_err());
    }

    fn setup() -> TestSetup {
        const DEFAULT_PROGRAMS: [Pubkey; 0] = [];
        const DEFAULT_PROGRAMS_PATH: &str = "programs/";
        const OUR_PROGRAM_ID: Pubkey =
            Pubkey::from_str_const("EFPnDWAebC2S6CdqWC9gj9vcxHv3Hiba6zQfv2LxZKfR");
        const OUR_PROGRAM_PATH: &str = "../target/deploy/solana_gamble_onchain.so";

        let mut lite_svm = LiteSVM::new().with_default_programs().with_sysvars();

        lite_svm
            .add_program_from_file(OUR_PROGRAM_ID, OUR_PROGRAM_PATH)
            .expect("Failed to load our program!");

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
            .expect("Failed to set creator account");

        setup
            .lite_svm
            .airdrop(&creator_account.pubkey(), 10_u64.pow(9))
            .expect("Failed to airdrop creator account  ");

        let bet_account = Keypair::new();

        let mut instruction_data = vec![0];

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
            &[&creator_account],
            setup.lite_svm.latest_blockhash(),
        );

        match setup.lite_svm.simulate_transaction(transaction) {
            Ok(res) => println!("Transaction succeeded with result: {:?}", res),
            Err(e) => panic!("Transaction failed: {}", e.err),
        }
    }
}
