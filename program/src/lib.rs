#![allow(unexpected_cfgs)]

use {
    crate::constants::{BASIS_POINT, DEFAULT_DECIMALS, FEE_BPS, FEE_WALLET, MINT_DEFAULT_SIZE},
    hexis_prediction_market_interface::{Prediction, PredictionInstruction},
    pinocchio::{
        account_info::AccountInfo,
        entrypoint,
        log::sol_log,
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvars::{rent::Rent, Sysvar},
        ProgramResult,
    },
};
mod ata_accessor;
mod constants;

use ata_accessor::*;

entrypoint!(process_instruction);

// Unpack the instruction data into a known instruction
pub fn unpack(input: &[u8]) -> Result<PredictionInstruction, ProgramError> {
    let (discriminator, rest) = input
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    // Each brace has error handling for each instruction parsing
    Ok(match discriminator {
        // Create doesn't have any instruction data, since it just initializes a prediction (for now)
        0 => PredictionInstruction::CreatePrediction {},
        1 => {
            let option = rest.get(0).ok_or(ProgramError::InvalidInstructionData)?;
            let amount = rest
                .get(1..9)
                .and_then(|slice| slice.try_into().ok())
                .map(u64::from_le_bytes)
                .ok_or(ProgramError::InvalidInstructionData)?;

            PredictionInstruction::PlaceBet {
                option: *option,
                amount,
            }
        }
        2 => {
            let winner = rest.get(0).ok_or(ProgramError::InvalidInstructionData)?;
            PredictionInstruction::EndPrediction { winner: *winner }
        }
        // Claim doesn't have any instruction data, since all that is needed is user token vault
        3 => PredictionInstruction::Claim,
        _ => return Err(ProgramError::InvalidInstructionData),
    })
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = unpack(instruction_data)?;

    match instruction {
        PredictionInstruction::CreatePrediction {} => {
            sol_log("Instruction: CreateBet");
            create(program_id, accounts)
        }
        PredictionInstruction::PlaceBet { option, amount } => {
            sol_log("Instruction: PlaceBet");
            place_bet(program_id, accounts, option, amount)
        }
        PredictionInstruction::EndPrediction { winner } => {
            sol_log("Instruction: SettleBet");
            end_prediction(program_id, accounts, winner)
        }
        PredictionInstruction::Claim => {
            sol_log("Instruction: Claim");
            claim(accounts)
        }
    }
}

/// Initializes a new prediction
fn create(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
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

    // Create prediction account
    pinocchio_system::instructions::CreateAccount {
        from: creator_account,
        to: prediction_account,
        lamports: Rent::get()?.minimum_balance(std::mem::size_of::<Prediction>()),
        space: std::mem::size_of::<Prediction>() as u64,
        owner: program_id,
    }
    .invoke()?;

    let prediction = unsafe {
        &mut *(prediction_account.try_borrow_mut_data()?.as_mut_ptr() as *mut Prediction)
    };

    // Create mint accounts
    pinocchio_system::instructions::CreateAccount {
        from: creator_account,
        to: mint_a_account,
        lamports: Rent::get()?.minimum_balance(MINT_DEFAULT_SIZE as usize),
        space: MINT_DEFAULT_SIZE as u64,
        owner: &constants::TOKEN_PROGRAM_2022,
    }
    .invoke()?;

    pinocchio_system::instructions::CreateAccount {
        from: creator_account,
        to: mint_b_account,
        lamports: Rent::get()?.minimum_balance(MINT_DEFAULT_SIZE as usize),
        space: MINT_DEFAULT_SIZE as u64,
        owner: &constants::TOKEN_PROGRAM_2022,
    }
    .invoke()?;

    // Initializes both mint accounts (but doesn't mint any tokens yet)
    pinocchio_token_2022::instructions::InitializeMint2 {
        mint: mint_a_account,
        decimals: DEFAULT_DECIMALS,
        mint_authority: prediction_account.key(),
        freeze_authority: Some(prediction_account.key()),
        token_program: &constants::TOKEN_PROGRAM_2022,
    }
    .invoke()?;

    pinocchio_token_2022::instructions::InitializeMint2 {
        mint: mint_b_account,
        decimals: DEFAULT_DECIMALS,
        mint_authority: prediction_account.key(),
        freeze_authority: Some(prediction_account.key()),
        token_program: &constants::TOKEN_PROGRAM_2022,
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

    // Initialize prediction data
    prediction.creator = *creator_account.key();
    prediction.total_token_a = 0;
    prediction.total_token_b = 0;
    prediction.winner = 0;
    prediction.gamble_token_a_mint = *mint_a_account.key();
    prediction.gamble_token_b_mint = *mint_b_account.key();

    Ok(())
}

/// Place bet on some side of the prediction, transferring SOL from user to pool and minting
/// the corresponding tokens to the user
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

    let creator_sol_account = accounts_iter
        .next()
        .ok_or(ProgramError::NotEnoughAccountKeys)?;

    let protocol_fee_account = accounts_iter
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

    let creator_fee = amount
        .checked_mul(FEE_BPS)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(BASIS_POINT)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    pinocchio_token_2022::instructions::Transfer {
        from: user_sol_account,
        to: creator_sol_account,
        authority: gambler_account,
        amount: creator_fee,
        token_program: &constants::TOKEN_PROGRAM,
    }
    .invoke()?;

    let protocol_fee = amount
        .checked_mul(FEE_BPS)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(BASIS_POINT)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    pinocchio_token_2022::instructions::Transfer {
        from: user_sol_account,
        to: protocol_fee_account,
        authority: gambler_account,
        amount: protocol_fee,
        token_program: &constants::TOKEN_PROGRAM,
    }
    .invoke()?;

    // Checking after transfers to avoid duplicate reference borrow
    let creator_sol_account_data = creator_sol_account.try_borrow_data()?;
    let protocol_fee_account_data = protocol_fee_account.try_borrow_data()?;

    if AtaAccessor::get_owner(&creator_sol_account_data) != prediction.creator {
        sol_log("Creator SOL account isn't owned by the prediction creator");
        return Err(ProgramError::IllegalOwner);
    }

    if AtaAccessor::get_owner(&protocol_fee_account_data) != FEE_WALLET {
        sol_log("Protocol fee account isn't owned by the fee wallet");
        return Err(ProgramError::IllegalOwner);
    }

    let total_fee = creator_fee
        .checked_add(protocol_fee)
        .ok_or(ProgramError::ArithmeticOverflow)?;

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
        amount: amount
            .checked_sub(total_fee)
            .ok_or(ProgramError::ArithmeticOverflow)?,
        token_program: &constants::TOKEN_PROGRAM,
    }
    .invoke()?;

    // Minting the corresponding tokens to the user
    pinocchio_token_2022::instructions::MintTo {
        mint: mint_account,
        account: user_token_account,
        mint_authority: prediction_account,
        amount: amount
            .checked_sub(total_fee)
            .ok_or(ProgramError::ArithmeticOverflow)?,
        token_program: &constants::TOKEN_PROGRAM_2022,
    }
    .invoke()?;

    let net_amount = amount
        .checked_sub(total_fee)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // This works because token has equivalent decimals, so 1 lamport = token (for the program)
    if option == 1 {
        prediction
            .total_token_a
            .checked_add(net_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    } else {
        prediction
            .total_token_b
            .checked_add(net_amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
    }

    Ok(())
}

/// Ends the prediction by setting the winner
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

    // Only the creator can end the predictions
    if creator_account.is_signer() && *creator_account.key() != prediction.creator {
        sol_log("Only the creator can settle the prediction");
        return Err(ProgramError::IllegalOwner);
    }

    // Check if the prediction has already been settled
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

    // Check if the prediction has been settled
    if prediction.winner == 0 {
        sol_log("Prediction has not been settled yet");
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if the winner option is valid
    if prediction.winner != 1 && prediction.winner != 2 {
        sol_log("Invalid winner option in prediction");
        return Err(ProgramError::InvalidAccountData);
    }

    let winner_mint = if prediction.winner == 1 {
        prediction.gamble_token_a_mint
    } else {
        prediction.gamble_token_b_mint
    };

    // Check if the user token account mint matches the winner mint
    if winner_mint != user_token_account_mint {
        sol_log("Winner mint doesn't match provided user token account");
        return Err(ProgramError::InvalidAccountData);
    }

    let user_token_amount = AtaAccessor::get_amount(&user_token_account.try_borrow_data()?);
    let winner_token_amount = if prediction.winner == 1 {
        prediction.total_token_a
    } else {
        prediction.total_token_b
    };

    let total_sol_deposited = prediction
        .total_token_a
        .checked_add(prediction.total_token_b)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    let amount_won = user_token_amount
        .checked_mul(total_sol_deposited)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(winner_token_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Burn all user tokens (so he can't claim again)
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
