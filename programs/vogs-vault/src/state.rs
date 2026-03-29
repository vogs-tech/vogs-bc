use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub authority: Pubkey,
    pub institution: Pubkey,
    pub mint: Pubkey,
    pub token_account: Pubkey,
    pub total_deposits: u64,
    pub liquidity_buffer_bps: u16,
    #[max_len(5)]
    pub allocations: Vec<ProtocolAllocation>,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct ProtocolAllocation {
    #[max_len(32)]
    pub name: String,
    pub target_bps: u16,
    pub current_bps: u16,
    pub apy_bps: u16,
}

#[account]
#[derive(InitSpace)]
pub struct DepositorRecord {
    pub vault: Pubkey,
    pub depositor: Pubkey,
    pub balance: u64,
    pub last_deposit_at: i64,
    pub bump: u8,
}
