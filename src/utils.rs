use {
    crate::constants::TOKEN_PROGRAM_2022,
    pinocchio::{account_info::AccountInfo, pubkey::Pubkey, ProgramResult},
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
