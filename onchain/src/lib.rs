use bytemuck::{Pod, Zeroable};
use pinocchio::{
    instruction::Instruction, ProgramResult, account_info::AccountInfo, entrypoint, msg, program_error::ProgramError, pubkey::{Pubkey, find_program_address}, sysvars::{Sysvar, rent::Rent}
};
mod token_account;
use token_account::*;

entrypoint!(process_instruction);

// Define the data structures for the program
#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct Bet {
    pub creator: Pubkey,
    pub gamble_token_a_mint: Pubkey,
    pub gamble_token_b_mint: Pubkey,
    pub gamble_vault_a: Pubkey,
    pub gamble_vault_b: Pubkey,
    pub total_amount: u64,
    pub winner: u8,
}

pub enum BetInstruction {
    CreateBet { amount: u64 },
    PlaceBet { option: u8, amount: u64 }, // 1 for option a, 2 for option b
    SettleBet { winner: u8 },             // 1 for option a, 2 for option b
    Withdraw {},
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
                Self::CreateBet { amount }
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
                Self::SettleBet { winner: *winner }
            }
            3 => Self::Withdraw,
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
        BetInstruction::CreateBet { amount } => {
            msg!("Instruction: CreateBet");
            create_bet(program_id, accounts, amount)
        }
        BetInstruction::PlaceBet { option, amount } => {
            msg!("Instruction: PlaceBet");
            place_bet(program_id, accounts, option, amount)
        }
        BetInstruction::SettleBet { winner } => {
            msg!("Instruction: SettleBet");
            settle_bet(program_id, accounts, winner)
        }
        BetInstruction::Withdraw {} => {
            msg!("Instruction: Withdraw");
            withdraw(accounts)
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

    let mint_a = find_program_address(&[bet_account, 1], program_id);
    let mint_b = find_program_address(&[bet_account, 2], program_id);

    pinocchio_token_2022::MintTo {
        mint: mint_a,
        destination: vault_a_account,
        authority: bet_account,
        amount: 10_u64.pow(9) * 10_u64.pow(6),
    };

    pinocchio_token_2022::instruction::MintTo {
        mint: mint_b,
        destination: vault_b_account,
        authority: bet_account,
        amount: 10_u64.pow(9) * 10_u64.pow(6),
    };

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
        mint: mint_a,
        system_program: todo!(),
        token_program: todo!(),
    }
    .invoke()?;
    pinocchio_associated_token_account::instructions::Create {
        funding_account: todo!(),
        account: todo!(),
        wallet: todo!(),
        mint: todo!(),
        system_program: todo!(),
        token_program: todo!(),
    }
    .invoke()?;

    let mut bet_data = bet_account.try_borrow_mut_data()?;
    let bet = bytemuck::from_bytes_mut::<Bet>(&mut bet_data);

    bet.creator = *creator_account.key();
    bet.total_amount = 0;
    bet.winner = 0;

    Ok(())
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

    if unsafe { *bet_account.owner() } != *program_id {
        msg!("Bet account not owned by the program");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut bet_data = bet_account.try_borrow_mut_data()?;
    let bet = bytemuck::from_bytes_mut::<Bet>(&mut bet_data);

    if bet.winner != 0 {
        msg!("Bet already settled");
        return Err(ProgramError::InvalidAccountData);
    }

    match option {
        1 => bet.option_a_votes += 1,
        2 => bet.option_b_votes += 1,
        _ => {
            msg!("Invalid option");
            return Err(ProgramError::InvalidInstructionData);
        }
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

    if unsafe { *bet_account.owner() } != *program_id {
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

    let winner_votes = if winner == 1 {
        bet.option_a_votes
    } else {
        bet.option_b_votes
    };

    let total_amount = bet.total_amount;

    // Transfer the funds to the winners
    for account in accounts_iter {
        let share = (total_amount as f64 / winner_votes as f64) as u64;
        *account.try_borrow_mut_lamports()? += share;
    }

    *creator_account.try_borrow_mut_lamports()? += bet_account.lamports();

    bet.is_initialized = 0;
    bet.creator = Pubkey::default();
    bet.option_a_votes = 0;
    bet.option_b_votes = 0;
    bet.total_amount = 0;

    Ok(())
}

// [signer, signer_token_account, token_mint, pool, vault_wsol, vault_token, token_program]
fn withdraw(accounts: &[AccountInfo]) -> ProgramResult {
    unsafe{

    let signer = accounts.get(0).ok_or(ProgramError::InvalidAccountData)?;
    let signer_token_account = accounts.get(1).ok_or(ProgramError::InvalidAccountData)?;
    let token_mint = accounts.get(2).ok_or(ProgramError::InvalidAccountData)?;
    let pool = accounts.get(3).ok_or(ProgramError::InvalidAccountData)?;
    let vault_wsol = accounts.get(4).ok_or(ProgramError::InvalidAccountData)?;
    let vault_token = accounts.get(5).ok_or(ProgramError::InvalidAccountData)?;
    let token_program = accounts.get(6).ok_or(ProgramError::InvalidAccountData)?;

    let token_amount = AtaAccessor::get_mint(signer_token_account.borrow_data_unchacked());

    let ix_take_token = Instruction{
        program_id: token_program.key() ,
        data: todo!(),
        accounts: todo!(),
    };

    invoke{

    }
    Ok(())
    }
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
            .expect("Failed to set creator account");

        setup
            .lite_svm
            .airdrop(&creator_account.pubkey(), 10_u64.pow(9))
            .expect("Failed to airdrop creator account  ");

        let bet_account = Keypair::new();

        // 0 = CreateBet instruction
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
