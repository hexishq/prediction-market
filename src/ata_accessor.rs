use pinocchio::pubkey::Pubkey;

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

    pub fn get_mint(data: &[u8]) -> Pubkey {
        data[Self::INDEXES.offset_mint..Self::INDEXES.offset_owner]
            .try_into()
            .unwrap()
    }

    pub fn get_owner(data: &[u8]) -> Pubkey {
        data[Self::INDEXES.offset_owner..Self::INDEXES.offset_amount]
            .try_into()
            .unwrap()
    }

    pub fn get_amount(data: &[u8]) -> u64 {
        u64::from_le_bytes(
            data[Self::INDEXES.offset_amount..Self::INDEXES.offset_delegate_option]
                .try_into()
                .unwrap(),
        )
    }
}
