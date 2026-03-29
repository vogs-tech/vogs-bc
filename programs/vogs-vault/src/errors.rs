use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Insufficient balance for withdrawal")]
    InsufficientBalance,
    #[msg("Allocation percentages must sum to 10,000 basis points (100%)")]
    InvalidAllocationSum,
    #[msg("Maximum of 5 protocol allocations allowed")]
    TooManyAllocations,
    #[msg("Unauthorized: caller is not the depositor")]
    Unauthorized,
}
