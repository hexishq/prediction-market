#![allow(unexpected_cfgs)]

use {
    crate::utils::initialize_mint,
    bytemuck::{Pod, Zeroable},
    pinocchio::{
        account_info::AccountInfo,
        entrypoint,
        log::sol_log,
        msg,
        program_error::ProgramError,
        pubkey::{find_program_address, Pubkey},
        sysvars::{rent::Rent, Sysvar},
        ProgramResult,
    },
};
mod ata_accessor;
mod constants;
mod utils;

use ata_accessor::*;

entrypoint!(process_instruction);

// Define the data structures for the program
#[repr(C, packed)]
#[derive(Copy, Clone, Zeroable, Pod)]
pub struct Prediction {
    pub creator: Pubkey,
    pub gamble_token_a_mint: Pubkey,
    pub gamble_token_b_mint: Pubkey,
    pub total_amount: u64,
    pub winner: u8,
    pub padding: [u8; 7],
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
            sol_log("Instruction: CreateBet");
            create(program_id, accounts, amount)
        }
        BetInstruction::PlaceBet { option, amount } => {
            sol_log("Instruction: PlaceBet");
            place_bet(program_id, accounts, option, amount)
        }
        BetInstruction::EndPrediction { winner } => {
            sol_log("Instruction: SettleBet");
            end_prediction(program_id, accounts, winner)
        }
        BetInstruction::Claim => {
            sol_log("Instruction: Claim");
            claim(accounts)
        }
    }
}

// Create a new prediction
fn create(program_id: &Pubkey, accounts: &[AccountInfo], _amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let creator_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let prediction_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let sol_vault_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let mint_a_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let mint_b_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let sol = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let system_program = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let token_program = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let (mint_a, _bump) = find_program_address(
        &[prediction_account.key(), &1_u64.to_le_bytes()],
        program_id,
    );

    let (mint_b, _bump) = find_program_address(
        &[prediction_account.key(), &2_u64.to_le_bytes()],
        program_id,
    );

    if mint_a_account.key() != &mint_a {
        sol_log("Mint A account does not match the derived address");
        return Err(ProgramError::InvalidAccountData);
    }

    if mint_b_account.key() != &mint_b {
        sol_log("Mint B account does not match the derived address");
        return Err(ProgramError::InvalidAccountData);
    }

    initialize_mint(9, mint_a_account, program_id, Some(program_id))?;
    initialize_mint(9, mint_b_account, program_id, Some(program_id))?;

    let rent = Rent::get()?;

    sol_log("Creating bet account");

    // Create prediction account
    pinocchio_system::instructions::CreateAccount {
        from: creator_account,
        to: prediction_account,
        lamports: rent.minimum_balance(std::mem::size_of::<Prediction>()),
        space: std::mem::size_of::<Prediction>() as u64,
        owner: program_id,
    }
    .invoke()?;

    // Create pool Wsol account
    pinocchio_associated_token_account::instructions::Create {
        funding_account: creator_account,
        account: sol_vault_account,
        wallet: prediction_account,
        mint: sol,
        system_program: system_program,
        token_program: token_program,
    }
    .invoke()?;

    let mut prediction_data = prediction_account.try_borrow_mut_data()?;

    let prediction =
        bytemuck::try_from_bytes_mut::<Prediction>(&mut prediction_data).map_err(|e| {
            sol_log(&format!("Failed to borrow prediction data: {e}"));
            ProgramError::InvalidAccountData
        })?;

    prediction.creator = *creator_account.key();
    prediction.total_amount = 0;
    prediction.winner = 0;
    prediction.gamble_token_a_mint = mint_a;
    prediction.gamble_token_b_mint = mint_b;

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

    let prediction_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let pool_sol_vault_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let user_sol_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let user_token_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let mint_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    if *prediction_account.owner() != *program_id {
        sol_log("Prediction account not owned by the program");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut prediction_data = prediction_account.try_borrow_mut_data()?;

    let prediction =
        bytemuck::try_from_bytes_mut::<Prediction>(&mut prediction_data).map_err(|e| {
            sol_log(&format!("Failed to deserialize prediction data: {e}"));
            ProgramError::InvalidAccountData
        })?;

    let mint_to_transfer = if option == 1 {
        prediction.gamble_token_a_mint
    } else {
        prediction.gamble_token_b_mint
    };

    if prediction.winner != 0 {
        sol_log("Prediction has already ended");
        return Err(ProgramError::InvalidAccountData);
    }

    if [1, 2].contains(&option) == false {
        sol_log("Invalid option");
        return Err(ProgramError::InvalidInstructionData);
    }

    let user_vault_data = user_token_account.try_borrow_data()?;
    let user_sol_account_data = user_sol_account.try_borrow_data()?;

    if AtaAccessor::get_mint(&user_vault_data) != mint_to_transfer {
        sol_log("User token account mint does not match the selected option");
        return Err(ProgramError::InvalidAccountData);
    }

    if AtaAccessor::get_amount(&user_sol_account_data) < amount {
        sol_log("Insufficient SOL balance in user account");
        return Err(ProgramError::InsufficientFunds);
    }

    // Sending SOL from user to pool vault
    pinocchio_token_2022::instructions::Transfer {
        from: user_sol_account,
        to: pool_sol_vault_account,
        authority: gambler_account,
        amount,
        token_program: &constants::TOKEN_PROGRAM,
    }
    .invoke()?;

    pinocchio_token_2022::instructions::MintTo {
        mint: mint_account,
        account: user_token_account,
        mint_authority: prediction_account,
        amount,
        token_program: &constants::TOKEN_PROGRAM_2022,
    }
    .invoke()?;

    // Transfer lamports from gambler to bet account
    pinocchio_system::instructions::Transfer {
        from: gambler_account,
        to: prediction_account,
        lamports: amount,
    }
    .invoke()?;

    prediction.total_amount += amount;

    Ok(())
}

// Ends the prediction by setting the winner
fn end_prediction(program_id: &Pubkey, accounts: &[AccountInfo], winner: u8) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let creator_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let prediction_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    if *prediction_account.owner() != *program_id {
        sol_log("Prediction account not owned by the program");
        return Err(ProgramError::IncorrectProgramId);
    }

    let mut prediction_data = prediction_account.try_borrow_mut_data()?;
    let prediction = bytemuck::from_bytes_mut::<Prediction>(&mut prediction_data);

    if creator_account.is_signer() && *creator_account.key() != prediction.creator {
        sol_log("Only the creator can settle the prediction");
        return Err(ProgramError::IllegalOwner);
    }

    if prediction.winner != 0 {
        sol_log("Prediction already settled");
        return Err(ProgramError::InvalidAccountData);
    }

    prediction.winner = winner;

    Ok(())
}

fn claim(accounts: &[AccountInfo]) -> ProgramResult {
    let mut accounts_iter = accounts.iter();

    let signer = accounts_iter
        .next()
        .ok_or(ProgramError::InvalidAccountData)?;

    let user_token_account = accounts_iter
        .next()
        .ok_or(ProgramError::InvalidAccountData)?;

    let mint_account = accounts_iter
        .next()
        .ok_or(ProgramError::InvalidAccountData)?;

    let pool_sol_vault = accounts_iter
        .next()
        .ok_or(ProgramError::InvalidAccountData)?;

    let prediction_account = accounts_iter
        .next()
        .ok_or(ProgramError::InvalidAccountData)?;

    let user_token_account_mint = AtaAccessor::get_mint(&user_token_account.try_borrow_data()?);
    let prediction_data = prediction_account.try_borrow_data()?;

    let prediction = bytemuck::try_from_bytes::<Prediction>(&prediction_data).map_err(|e| {
        sol_log(&format!("Failed to deserialize prediction account: {e}"));
        ProgramError::InvalidAccountData
    })?;

    if prediction.winner == 0 {
        sol_log("Prediction has not been settled yet");
        return Err(ProgramError::InvalidAccountData);
    }

    if prediction.winner != 1 && prediction.winner != 2 {
        sol_log("Invalid winner option in prediction");
        return Err(ProgramError::InvalidAccountData);
    }

    let winner_mint = if prediction.winner == 1 {
        prediction.gamble_token_a_mint
    } else {
        prediction.gamble_token_b_mint
    };

    if winner_mint != user_token_account_mint {
        sol_log("Winner mint doesn't match provided user token account");
        return Err(ProgramError::InvalidAccountData);
    }

    let user_token_amount = AtaAccessor::get_amount(&user_token_account.try_borrow_data()?);

    let amount_won = user_token_amount
        .checked_div(prediction.total_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Burn all user tokens
    pinocchio_token_2022::instructions::Burn {
        mint: mint_account,
        account: user_token_account,
        authority: signer,
        amount: user_token_amount,
        token_program: &constants::TOKEN_PROGRAM_2022,
    }
    .invoke()?;

    // Send won SOL to the user
    pinocchio_token_2022::instructions::Transfer {
        from: pool_sol_vault,
        to: signer,
        authority: prediction_account,
        amount: amount_won,
        token_program: &constants::TOKEN_PROGRAM,
    }
    .invoke()?;

    Ok(())
}
