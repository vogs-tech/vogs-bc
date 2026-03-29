use anchor_lang::prelude::*;

#[event]
pub struct PositionLiquidated {
    pub position: Pubkey,
    pub liquidator: Pubkey,
    pub collateral_amount: u64,
    pub credit_drawn: u64,
    pub ltv_bps: u16,
}

#[event]
pub struct MarginCallAlert {
    pub position: Pubkey,
    pub current_ltv_bps: u16,
    pub margin_call_ltv_bps: u16,
    pub collateral_value: u64,
    pub credit_drawn: u64,
}
