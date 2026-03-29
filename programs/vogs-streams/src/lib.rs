use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, TransferChecked};
use anchor_spl::token_interface::{Mint, TokenAccount};

declare_id!("VStrm1111111111111111111111111111111111111");

pub mod state;
pub mod errors;

use errors::StreamError;
use state::{Escrow, Milestone, MilestoneStatus, Stream, StreamStatus};

#[program]
pub mod vogs_streams {
    use super::*;

    pub fn create_stream(
        ctx: Context<CreateStream>,
        total_amount: u64,
        start_time: i64,
        end_time: i64,
        nonce: u64,
    ) -> Result<()> {
        require!(end_time > start_time, StreamError::InvalidTimeRange);

        let rate_per_second = total_amount
            .checked_div((end_time - start_time) as u64)
            .unwrap();

        // Transfer total amount to escrow
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.sender_token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.escrow_token_account.to_account_info(),
            authority: ctx.accounts.sender.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token_2022::transfer_checked(cpi_ctx, total_amount, ctx.accounts.mint.decimals)?;

        let stream = &mut ctx.accounts.stream;
        stream.sender = ctx.accounts.sender.key();
        stream.recipient = ctx.accounts.recipient.key();
        stream.mint = ctx.accounts.mint.key();
        stream.escrow_token_account = ctx.accounts.escrow_token_account.key();
        stream.total_amount = total_amount;
        stream.withdrawn_amount = 0;
        stream.rate_per_second = rate_per_second;
        stream.start_time = start_time;
        stream.end_time = end_time;
        stream.last_withdraw_time = start_time;
        stream.paused_at = 0;
        stream.total_paused_duration = 0;
        stream.status = StreamStatus::Active;
        stream.nonce = nonce;
        stream.bump = ctx.bumps.stream;

        msg!("Stream created: {} tokens over {} seconds", total_amount, end_time - start_time);
        Ok(())
    }

    pub fn withdraw_from_stream(ctx: Context<WithdrawFromStream>) -> Result<()> {
        let stream = &mut ctx.accounts.stream;
        require!(stream.status == StreamStatus::Active, StreamError::StreamNotActive);

        let now = Clock::get()?.unix_timestamp;
        let effective_time = now.min(stream.end_time);
        let elapsed = (effective_time - stream.last_withdraw_time) as u64;
        let withdrawable = elapsed
            .checked_mul(stream.rate_per_second)
            .unwrap()
            .min(stream.total_amount - stream.withdrawn_amount);

        require!(withdrawable > 0, StreamError::NothingToWithdraw);

        // Transfer from escrow to recipient (PDA signer)
        let sender_key = stream.sender;
        let nonce_bytes = stream.nonce.to_le_bytes();
        let seeds = &[
            b"stream",
            sender_key.as_ref(),
            nonce_bytes.as_ref(),
            &[stream.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = TransferChecked {
            from: ctx.accounts.escrow_token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.stream.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token_2022::transfer_checked(cpi_ctx, withdrawable, ctx.accounts.mint.decimals)?;

        stream.withdrawn_amount = stream.withdrawn_amount.checked_add(withdrawable).unwrap();
        stream.last_withdraw_time = effective_time;

        msg!("Withdrew {} tokens from stream", withdrawable);
        Ok(())
    }

    pub fn pause_stream(ctx: Context<ManageStream>) -> Result<()> {
        let stream = &mut ctx.accounts.stream;
        require!(stream.status == StreamStatus::Active, StreamError::StreamNotActive);

        stream.status = StreamStatus::Paused;
        stream.paused_at = Clock::get()?.unix_timestamp;

        msg!("Stream paused");
        Ok(())
    }

    pub fn resume_stream(ctx: Context<ManageStream>) -> Result<()> {
        let stream = &mut ctx.accounts.stream;
        require!(stream.status == StreamStatus::Paused, StreamError::StreamNotPaused);

        let now = Clock::get()?.unix_timestamp;
        let paused_duration = now - stream.paused_at;
        stream.total_paused_duration += paused_duration;
        stream.last_withdraw_time += paused_duration;
        stream.status = StreamStatus::Active;
        stream.paused_at = 0;

        msg!("Stream resumed");
        Ok(())
    }

    pub fn cancel_stream(ctx: Context<CancelStream>) -> Result<()> {
        let stream = &mut ctx.accounts.stream;
        let now = Clock::get()?.unix_timestamp;

        // Calculate earned amount for recipient
        let effective_time = now.min(stream.end_time);
        let elapsed = (effective_time - stream.start_time) as u64;
        let total_earned = elapsed
            .checked_mul(stream.rate_per_second)
            .unwrap()
            .min(stream.total_amount);
        let owed_to_recipient = total_earned.saturating_sub(stream.withdrawn_amount);
        let return_to_sender = stream.total_amount - stream.withdrawn_amount - owed_to_recipient;

        let sender_key = stream.sender;
        let nonce_bytes = stream.nonce.to_le_bytes();
        let seeds = &[
            b"stream",
            sender_key.as_ref(),
            nonce_bytes.as_ref(),
            &[stream.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        // Transfer owed amount to recipient
        if owed_to_recipient > 0 {
            let cpi_accounts = TransferChecked {
                from: ctx.accounts.escrow_token_account.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: ctx.accounts.stream.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            );
            token_2022::transfer_checked(cpi_ctx, owed_to_recipient, ctx.accounts.mint.decimals)?;
        }

        // Return remaining to sender
        if return_to_sender > 0 {
            let cpi_accounts = TransferChecked {
                from: ctx.accounts.escrow_token_account.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.sender_token_account.to_account_info(),
                authority: ctx.accounts.stream.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer_seeds,
            );
            token_2022::transfer_checked(cpi_ctx, return_to_sender, ctx.accounts.mint.decimals)?;
        }

        stream.status = StreamStatus::Cancelled;
        msg!("Stream cancelled: {} to recipient, {} returned to sender", owed_to_recipient, return_to_sender);
        Ok(())
    }

    pub fn create_escrow(
        ctx: Context<CreateEscrow>,
        milestones: Vec<Milestone>,
        nonce: u64,
    ) -> Result<()> {
        let total: u64 = milestones.iter().map(|m| m.amount).sum();

        // Transfer total to escrow
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.sender_token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.escrow_token_account.to_account_info(),
            authority: ctx.accounts.sender.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token_2022::transfer_checked(cpi_ctx, total, ctx.accounts.mint.decimals)?;

        let escrow = &mut ctx.accounts.escrow;
        escrow.sender = ctx.accounts.sender.key();
        escrow.recipient = ctx.accounts.recipient.key();
        escrow.mint = ctx.accounts.mint.key();
        escrow.escrow_token_account = ctx.accounts.escrow_token_account.key();
        escrow.milestones = milestones;
        escrow.total_amount = total;
        escrow.released_amount = 0;
        escrow.nonce = nonce;
        escrow.bump = ctx.bumps.escrow;

        msg!("Escrow created with {} milestones, total {}", escrow.milestones.len(), total);
        Ok(())
    }

    pub fn release_milestone(
        ctx: Context<ReleaseMilestone>,
        milestone_index: u8,
    ) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        let milestone = &mut escrow.milestones[milestone_index as usize];
        require!(milestone.status == MilestoneStatus::Pending, StreamError::MilestoneAlreadyReleased);

        // Check oracle condition (non-zero first byte)
        let oracle_data = ctx.accounts.oracle_condition.try_borrow_data()?;
        require!(!oracle_data.is_empty() && oracle_data[0] != 0, StreamError::MilestoneConditionNotMet);

        let amount = milestone.amount;
        milestone.status = MilestoneStatus::Released;
        escrow.released_amount = escrow.released_amount.checked_add(amount).unwrap();

        // Transfer milestone amount to recipient
        let sender_key = escrow.sender;
        let nonce_bytes = escrow.nonce.to_le_bytes();
        let seeds = &[
            b"escrow",
            sender_key.as_ref(),
            nonce_bytes.as_ref(),
            &[escrow.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = TransferChecked {
            from: ctx.accounts.escrow_token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.recipient_token_account.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token_2022::transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)?;

        msg!("Milestone {} released: {} tokens", milestone_index, amount);
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(total_amount: u64, start_time: i64, end_time: i64, nonce: u64)]
pub struct CreateStream<'info> {
    #[account(
        init,
        payer = sender,
        space = 8 + Stream::INIT_SPACE,
        seeds = [b"stream", sender.key().as_ref(), nonce.to_le_bytes().as_ref()],
        bump,
    )]
    pub stream: Account<'info, Stream>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub escrow_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: Recipient pubkey
    pub recipient: UncheckedAccount<'info>,
    #[account(mut)]
    pub sender: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawFromStream<'info> {
    #[account(
        mut,
        seeds = [b"stream", stream.sender.as_ref(), stream.nonce.to_le_bytes().as_ref()],
        bump = stream.bump,
        has_one = recipient,
    )]
    pub stream: Account<'info, Stream>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub escrow_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,
    pub recipient: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct ManageStream<'info> {
    #[account(
        mut,
        seeds = [b"stream", stream.sender.as_ref(), stream.nonce.to_le_bytes().as_ref()],
        bump = stream.bump,
        has_one = sender,
    )]
    pub stream: Account<'info, Stream>,
    pub sender: Signer<'info>,
}

#[derive(Accounts)]
pub struct CancelStream<'info> {
    #[account(
        mut,
        seeds = [b"stream", stream.sender.as_ref(), stream.nonce.to_le_bytes().as_ref()],
        bump = stream.bump,
        has_one = sender,
    )]
    pub stream: Account<'info, Stream>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub escrow_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,
    pub sender: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
#[instruction(milestones: Vec<Milestone>, nonce: u64)]
pub struct CreateEscrow<'info> {
    #[account(
        init,
        payer = sender,
        space = 8 + Escrow::INIT_SPACE,
        seeds = [b"escrow", sender.key().as_ref(), nonce.to_le_bytes().as_ref()],
        bump,
    )]
    pub escrow: Account<'info, Escrow>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub escrow_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: Recipient pubkey
    pub recipient: UncheckedAccount<'info>,
    #[account(mut)]
    pub sender: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(milestone_index: u8)]
pub struct ReleaseMilestone<'info> {
    #[account(
        mut,
        seeds = [b"escrow", escrow.sender.as_ref(), escrow.nonce.to_le_bytes().as_ref()],
        bump = escrow.bump,
    )]
    pub escrow: Account<'info, Escrow>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub escrow_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: Oracle condition account — first byte non-zero means condition met
    pub oracle_condition: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token2022>,
}
