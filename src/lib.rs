use bytemuck::{Pod, Zeroable};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{rent::Rent, Sysvar},
};

entrypoint!(process_instruction);

// Define the data structures for the program
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Bet {
    pub is_initialized: u8, // 1 = initialized, 0 = not initialized
    pub winner: u8,         // 0 = not decided, 1 = option a, 2 = option b
    pub creator: Pubkey,
    pub option_a_votes: u64,
    pub option_b_votes: u64,
    pub total_amount: u64,
}

pub enum BetInstruction {
    CreateBet { amount: u64 },
    PlaceBet { option: u8 },  // 1 for option a, 2 for option b
    SettleBet { winner: u8 }, // 1 for option a, 2 for option b
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
                Self::PlaceBet { option: *option }
            }
            2 => {
                let winner = rest.get(0).ok_or(ProgramError::InvalidInstructionData)?;
                Self::SettleBet { winner: *winner }
            }
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
        BetInstruction::PlaceBet { option } => {
            msg!("Instruction: PlaceBet");
            place_bet(program_id, accounts, option)
        }
        BetInstruction::SettleBet { winner } => {
            msg!("Instruction: SettleBet");
            settle_bet(program_id, accounts, winner)
        }
    }
}

// Create a new bet
fn create_bet(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let creator_account = next_account_info(accounts_iter)?;
    let bet_account = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    // Create the bet account
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(std::mem::size_of::<Bet>());

    invoke(
        &system_instruction::create_account(
            creator_account.key,
            bet_account.key,
            rent_lamports,
            std::mem::size_of::<Bet>() as u64,
            program_id,
        ),
        &[
            creator_account.clone(),
            bet_account.clone(),
            system_program.clone(),
        ],
    )?;

    // Initialize the bet account
    let mut bet_data = bet_account.try_borrow_mut_data()?;
    let bet = bytemuck::from_bytes_mut::<Bet>(&mut bet_data);

    bet.is_initialized = 1;
    bet.creator = *creator_account.key;
    bet.option_a_votes = 0;
    bet.option_b_votes = 0;
    bet.total_amount = 0;
    bet.winner = 0;

    // Transfer the initial amount to the bet account
    invoke(
        &system_instruction::transfer(creator_account.key, bet_account.key, amount),
        &[
            creator_account.clone(),
            bet_account.clone(),
            system_program.clone(),
        ],
    )?;

    bet.total_amount = amount;

    Ok(())
}

// Place a bet on one of the alternatives
fn place_bet(program_id: &Pubkey, accounts: &[AccountInfo], option: u8) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let better_account = next_account_info(accounts_iter)?;
    let bet_account = next_account_info(accounts_iter)?;

    // Check if the bet account is owned by the program
    if bet_account.owner != program_id {
        msg!("Bet account not owned by the program");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Get the bet data
    let mut bet_data = bet_account.try_borrow_mut_data()?;
    let bet = bytemuck::from_bytes_mut::<Bet>(&mut bet_data);

    // Check if the bet is already settled
    if bet.winner != 0 {
        msg!("Bet already settled");
        return Err(ProgramError::InvalidAccountData);
    }

    // Update the bet data
    match option {
        1 => bet.option_a_votes += 1,
        2 => bet.option_b_votes += 1,
        _ => {
            msg!("Invalid option");
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    // Transfer the bet amount to the bet account
    invoke(
        &system_instruction::transfer(better_account.key, bet_account.key, bet.total_amount),
        &[better_account.clone(), bet_account.clone()],
    )?;

    bet.total_amount += bet.total_amount;

    Ok(())
}

// Settle the bet and distribute the funds
fn settle_bet(program_id: &Pubkey, accounts: &[AccountInfo], winner: u8) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let creator_account = next_account_info(accounts_iter)?;
    let bet_account = next_account_info(accounts_iter)?;

    // Check if the bet account is owned by the program
    if bet_account.owner != program_id {
        msg!("Bet account not owned by the program");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Get the bet data
    let mut bet_data = bet_account.try_borrow_mut_data()?;
    let bet = bytemuck::from_bytes_mut::<Bet>(&mut bet_data);

    // Check if the caller is the creator of the bet
    if *creator_account.key != bet.creator {
        msg!("Only the creator can settle the bet");
        return Err(ProgramError::IllegalOwner);
    }

    // Check if the bet is already settled
    if bet.winner != 0 {
        msg!("Bet already settled");
        return Err(ProgramError::InvalidAccountData);
    }

    // Set the winner
    bet.winner = winner;

    // Distribute the funds
    let (winner_votes, loser_votes) = if winner == 1 {
        (bet.option_a_votes, bet.option_b_votes)
    } else {
        (bet.option_b_votes, bet.option_a_votes)
    };

    let total_amount = bet.total_amount;

    // Transfer the funds to the winners
    for account in accounts_iter {
        let _account_data = account.try_borrow_mut_data()?;
        let mut lamports = account.lamports.borrow_mut();

        let share = (total_amount as f64 / winner_votes as f64) as u64;
        **lamports += share;
    }

    // Transfer the remaining funds to the creator
    let mut creator_lamports = creator_account.lamports.borrow_mut();
    **creator_lamports += bet_account.lamports();

    // Clear the bet account data
    bet.is_initialized = 0;
    bet.creator = Pubkey::default();
    bet.option_a_votes = 0;
    bet.option_b_votes = 0;
    bet.total_amount = 0;
    bet.winner = 0;

    Ok(())
}
