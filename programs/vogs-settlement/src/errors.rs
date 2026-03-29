use anchor_lang::prelude::*;

#[error_code]
pub enum SettlementError {
    #[msg("Oracle price is stale or missing (older than 300 seconds)")]
    StaleOrMissingOracle,
    #[msg("Payment is not in Created status")]
    InvalidPaymentStatus,
}
