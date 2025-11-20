use bytemuck::{Pod, Zeroable};
use pinocchio::{
    account_info::AccountInfo, entrypoint, msg, program_error::ProgramError, pubkey::Pubkey,
    ProgramResult,
};

entrypoint!(process_instruction);

// Define the data structures for the program
#[repr(C)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct Bet {
    pub creator: Pubkey,
    pub option_a_votes: u64,
    pub option_b_votes: u64,
    pub total_amount: u64,
    pub is_initialized: u8, // 1 = initialized, 0 = not initialized
    pub winner: u8,         // 0 = not decided, 1 = option a, 2 = option b
    pub padding: [u8; 14],  // Padding to ensure alignment
}

pub enum BetInstruction {
    CreateBet { amount: u64 },
    PlaceBet { option: u8, amount: u64 }, // 1 for option a, 2 for option b
    SettleBet { winner: u8 },             // 1 for option a, 2 for option b
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
    }
}

// Create a new bet
fn create_bet(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let creator_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;
    let bet_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    pinocchio_system::create_account_with_minimum_balance(
        bet_account,
        std::mem::size_of::<Bet>() as usize,
        program_id,
        creator_account,
        None,
    )?;

    let mut bet_data = bet_account.try_borrow_mut_data()?;
    let bet = bytemuck::from_bytes_mut::<Bet>(&mut bet_data);

    bet.is_initialized = 1;
    bet.creator = *creator_account.key();
    bet.option_a_votes = 0;
    bet.option_b_votes = 0;
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

    if bet_account.owner() != program_id {
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

    pinocchio_system::instructions::Transfer {
        from: gambler_account,
        to: bet_account,
        lamports: amount,
    }
    .invoke()?;

    bet.total_amount += bet.total_amount;

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

    if bet_account.owner() != program_id {
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

    // Distribute funds
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
    bet.winner = 0;

    Ok(())
}
