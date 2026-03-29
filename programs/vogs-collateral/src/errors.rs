use anchor_lang::prelude::*;

#[error_code]
pub enum CollateralError {
    #[msg("Draw would exceed maximum loan-to-value ratio")]
    LtvExceeded,
    #[msg("Withdrawal would breach maximum loan-to-value ratio")]
    WithdrawalWouldBreachLtv,
    #[msg("Position LTV is below liquidation threshold — not liquidatable")]
    PositionNotLiquidatable,
    #[msg("Position is not active")]
    PositionNotActive,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Failed to read oracle price data")]
    OracleError,
}
