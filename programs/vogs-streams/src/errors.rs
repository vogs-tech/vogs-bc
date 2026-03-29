use anchor_lang::prelude::*;

#[error_code]
pub enum StreamError {
    #[msg("End time must be after start time")]
    InvalidTimeRange,
    #[msg("Milestone oracle condition not met (first byte is zero)")]
    MilestoneConditionNotMet,
    #[msg("Stream is not in Active status")]
    StreamNotActive,
    #[msg("Stream is not in Paused status")]
    StreamNotPaused,
    #[msg("Nothing to withdraw — no tokens have accrued")]
    NothingToWithdraw,
    #[msg("Milestone has already been released")]
    MilestoneAlreadyReleased,
}
