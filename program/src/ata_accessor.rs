use pinocchio::{msg, program_error::ProgramError, pubkey::Pubkey};

pub struct AtaAccessor;

pub struct AtaIndexes {
    offset_mint: usize,
    offset_owner: usize,
    offset_amount: usize,
    offset_delegate_option: usize,
}
impl AtaAccessor {
    pub const INDEXES: AtaIndexes = AtaIndexes {
        offset_mint: 0,
        offset_owner: 32,
        offset_amount: 64,
        offset_delegate_option: 72,
    };

    pub fn get_mint(data: &[u8]) -> Result<Pubkey, ProgramError> {
        data.get(Self::INDEXES.offset_mint..Self::INDEXES.offset_owner)
            .ok_or(ProgramError::InvalidInstructionData)?
            .try_into()
            .map_err(|_| {
                msg!("Failed to parse mint data into Pubkey");
                ProgramError::InvalidInstructionData
            })
    }

    pub fn get_owner(data: &[u8]) -> Result<Pubkey, ProgramError> {
        data.get(Self::INDEXES.offset_owner..Self::INDEXES.offset_amount)
            .ok_or(ProgramError::InvalidInstructionData)?
            .try_into()
            .map_err(|_| {
                msg!("Failed to parse owner data into Pubkey");
                ProgramError::InvalidInstructionData
            })
    }

    pub fn get_amount(data: &[u8]) -> Result<u64, ProgramError> {
        Ok(u64::from_le_bytes(
            data.get(Self::INDEXES.offset_amount..Self::INDEXES.offset_delegate_option)
                .ok_or(ProgramError::InvalidInstructionData)?
                .try_into()
                .map_err(|_| {
                    msg!("Failed to parse amount data into u64");
                    ProgramError::InvalidInstructionData
                })?,
        ))
    }
}
