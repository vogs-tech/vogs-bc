use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum StreamStatus {
    Active,
    Paused,
    Cancelled,
    Completed,
}

#[account]
#[derive(InitSpace)]
pub struct Stream {
    pub sender: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub escrow_token_account: Pubkey,
    pub total_amount: u64,
    pub withdrawn_amount: u64,
    pub rate_per_second: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub last_withdraw_time: i64,
    pub paused_at: i64,
    pub total_paused_duration: i64,
    pub status: StreamStatus,
    pub nonce: u64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum MilestoneStatus {
    Pending,
    Released,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct Milestone {
    pub description_hash: [u8; 32],
    pub amount: u64,
    pub oracle_condition: Pubkey,
    pub status: MilestoneStatus,
}

#[account]
#[derive(InitSpace)]
pub struct Escrow {
    pub sender: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub escrow_token_account: Pubkey,
    #[max_len(10)]
    pub milestones: Vec<Milestone>,
    pub total_amount: u64,
    pub released_amount: u64,
    pub nonce: u64,
    pub bump: u8,
}
