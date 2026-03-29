use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct HookConfig {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub gatekeeper_network: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct BlocklistEntry {
    pub mint: Pubkey,
    pub wallet: Pubkey,
    pub blocked_at: i64,
    pub bump: u8,
}
