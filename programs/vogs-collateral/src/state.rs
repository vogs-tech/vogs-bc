use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum PositionStatus {
    Active,
    Liquidated,
    Closed,
}

#[account]
#[derive(InitSpace)]
pub struct CollateralPosition {
    pub owner: Pubkey,
    pub collateral_mint: Pubkey,
    pub credit_mint: Pubkey,
    pub collateral_escrow: Pubkey,
    pub pyth_price_account: Pubkey,
    pub collateral_amount: u64,
    pub collateral_decimals: u8,
    pub credit_drawn: u64,
    pub max_ltv_bps: u16,
    pub margin_call_ltv_bps: u16,
    pub liquidation_ltv_bps: u16,
    pub status: PositionStatus,
    pub created_at: i64,
    pub nonce: u64,
    pub bump: u8,
}
