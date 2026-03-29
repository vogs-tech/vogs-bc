use anchor_lang::prelude::*;

#[event]
pub struct PaymentSettled {
    pub payment: Pubkey,
    pub source_amount: u64,
    pub dest_amount: u64,
    pub fx_rate: u64,
    pub settled_at: i64,
}
