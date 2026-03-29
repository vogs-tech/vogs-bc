use anchor_lang::prelude::*;

#[error_code]
pub enum HookError {
    #[msg("Sender does not have a valid KYC credential (Civic Pass)")]
    SenderNotKyc,
    #[msg("Receiver does not have a valid KYC credential (Civic Pass)")]
    ReceiverNotKyc,
    #[msg("Wallet is on the institution blocklist")]
    WalletBlocked,
    #[msg("Unauthorized: only the hook authority can perform this action")]
    Unauthorized,
}
