use pinocchio::pubkey::{Pubkey, PUBKEY_BYTES};
/// AssociatedTokenAccountLayout {
///     publicKey('mint'),
///     publicKey('owner'),
///     u64('amount'),
///     u32('delegateOption'),
///     publicKey('delegate'),
///     u8('state'),
///     u32('isNativeOption'),
///     u64('isNative'),
///     u64('delegatedAmount'),
///     u32('closeAuthorityOption'),
///     publicKey('closeAuthority'),
/// };
///
pub struct AtaAccessor {}
pub struct AtaIndexes {
    offset_mint: usize,
    offset_owner: usize,
    offset_amount: usize,
    offset_delegate_option: usize,
    offset_delegate: usize,
    offset_state: usize,
    offset_is_native_option: usize,
    offset_is_native: usize,
    offset_delegated_amount: usize,
    offset_close_authority_option: usize,
    offset_close_authority: usize,
}
impl AtaAccessor {
    pub const INDEXES: AtaIndexes = AtaIndexes {
        offset_mint: 0,
        offset_owner: 32,
        offset_amount: 64,
        offset_delegate_option: 72,
        offset_delegate: 76,
        offset_state: 108,
        offset_is_native_option: 109,
        offset_is_native: 113,
        offset_delegated_amount: 121,
        offset_close_authority_option: 129,
        offset_close_authority: 133,
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
    pub fn get_delegate_option(data: &[u8]) -> &[u8] {
        &data[Self::INDEXES.offset_delegate_option..Self::INDEXES.offset_delegate]
    }
    pub fn get_delegate(data: &[u8]) -> Pubkey {
        data[Self::INDEXES.offset_delegate..Self::INDEXES.offset_state]
            .try_into()
            .unwrap()
    }
    pub fn get_delegated_amount(data: &[u8]) -> u64 {
        u64::from_le_bytes(
            data[Self::INDEXES.offset_delegated_amount
                ..Self::INDEXES.offset_close_authority_option]
                .try_into()
                .unwrap(),
        )
    }
    pub fn get_state(data: &[u8]) -> &[u8] {
        &data[Self::INDEXES.offset_state..Self::INDEXES.offset_is_native_option]
    }
    pub fn get_is_native_option(data: &[u8]) -> &[u8] {
        &data[Self::INDEXES.offset_is_native_option..Self::INDEXES.offset_is_native]
    }
    pub fn get_is_native(data: &[u8]) -> &[u8] {
        &data[Self::INDEXES.offset_is_native..Self::INDEXES.offset_delegated_amount]
    }
    pub fn get_close_authority_option(data: &[u8]) -> &[u8] {
        &data[Self::INDEXES.offset_close_authority_option..Self::INDEXES.offset_close_authority]
    }
    pub fn get_close_authority(data: &[u8]) -> Pubkey {
        data[Self::INDEXES.offset_close_authority
            ..Self::INDEXES.offset_close_authority + PUBKEY_BYTES]
            .try_into()
            .unwrap()
    }
}
