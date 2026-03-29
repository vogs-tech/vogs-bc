use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum PaymentStatus {
    Created,
    Settled,
}

#[account]
#[derive(InitSpace)]
pub struct Payment {
    pub sender: Pubkey,
    pub recipient: Pubkey,
    pub source_mint: Pubkey,
    pub dest_mint: Pubkey,
    pub source_amount: u64,
    pub dest_amount: u64,
    pub fx_rate: u64,
    pub fx_decimals: u8,
    pub travel_rule_hash: [u8; 32],
    pub status: PaymentStatus,
    pub created_at: i64,
    pub settled_at: i64,
    pub nonce: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct OraclePrice {
    pub source_mint: Pubkey,
    pub dest_mint: Pubkey,
    pub price: u64,
    pub decimals: u8,
    pub updated_at: i64,
    pub authority: Pubkey,
    pub bump: u8,
}
