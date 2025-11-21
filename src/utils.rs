use {
    crate::constants::{BASIS_POINT, TOKEN_PROGRAM_2022},
    pinocchio::{
        account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
    },
};

pub fn initialize_mint(
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

pub fn transfer_fee(
    amount: u64,
    fee_bps: u64,
    from: &AccountInfo,
    to: &AccountInfo,
    authority: &AccountInfo,
    token_program: &Pubkey,
) -> Result<u64, ProgramError> {
    let amount_to_transfer = amount
        .checked_mul(fee_bps)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(BASIS_POINT)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    pinocchio_token_2022::instructions::Transfer {
        from,
        to,
        authority,
        amount: amount_to_transfer,
        token_program,
    }
    .invoke()?;

    Ok(amount_to_transfer)
}
