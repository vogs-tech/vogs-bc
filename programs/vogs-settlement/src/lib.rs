use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, TransferChecked};
use anchor_spl::token_interface::{Mint, TokenAccount};

declare_id!("VSett1e111111111111111111111111111111111111");

pub mod state;
pub mod errors;
pub mod events;

use errors::SettlementError;
use events::PaymentSettled;
use state::{OraclePrice, Payment, PaymentStatus};

const ORACLE_STALENESS_SECONDS: i64 = 300;

#[program]
pub mod vogs_settlement {
    use super::*;

    pub fn update_oracle(
        ctx: Context<UpdateOracle>,
        price: u64,
        decimals: u8,
    ) -> Result<()> {
        let oracle = &mut ctx.accounts.oracle_price;
        oracle.source_mint = ctx.accounts.source_mint.key();
        oracle.dest_mint = ctx.accounts.dest_mint.key();
        oracle.price = price;
        oracle.decimals = decimals;
        oracle.updated_at = Clock::get()?.unix_timestamp;
        oracle.authority = ctx.accounts.authority.key();
        oracle.bump = ctx.bumps.oracle_price;
        Ok(())
    }

    pub fn create_payment(
        ctx: Context<CreatePayment>,
        amount: u64,
        travel_rule_hash: [u8; 32],
        nonce: u64,
    ) -> Result<()> {
        let oracle = &ctx.accounts.oracle_price;
        let now = Clock::get()?.unix_timestamp;

        // Verify oracle freshness
        require!(
            now - oracle.updated_at <= ORACLE_STALENESS_SECONDS,
            SettlementError::StaleOrMissingOracle
        );

        let payment = &mut ctx.accounts.payment;
        payment.sender = ctx.accounts.sender.key();
        payment.recipient = ctx.accounts.recipient.key();
        payment.source_mint = ctx.accounts.source_mint.key();
        payment.dest_mint = ctx.accounts.dest_mint.key();
        payment.source_amount = amount;
        payment.dest_amount = amount
            .checked_mul(oracle.price)
            .unwrap()
            .checked_div(10u64.pow(oracle.decimals as u32))
            .unwrap();
        payment.fx_rate = oracle.price;
        payment.fx_decimals = oracle.decimals;
        payment.travel_rule_hash = travel_rule_hash;
        payment.status = PaymentStatus::Created;
        payment.created_at = now;
        payment.settled_at = 0;
        payment.nonce = nonce;
        payment.bump = ctx.bumps.payment;

        msg!("Payment created: {} -> {}", amount, payment.dest_amount);
        Ok(())
    }

    pub fn execute_settlement(ctx: Context<ExecuteSettlement>) -> Result<()> {
        let payment = &mut ctx.accounts.payment;
        require!(
            payment.status == PaymentStatus::Created,
            SettlementError::InvalidPaymentStatus
        );

        // Transfer source tokens from sender to protocol
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.sender_source_ata.to_account_info(),
            mint: ctx.accounts.source_mint.to_account_info(),
            to: ctx.accounts.protocol_source_ata.to_account_info(),
            authority: ctx.accounts.relayer.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        token_2022::transfer_checked(
            cpi_ctx,
            payment.source_amount,
            ctx.accounts.source_mint.decimals,
        )?;

        // Transfer dest tokens from protocol to recipient
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.protocol_dest_ata.to_account_info(),
            mint: ctx.accounts.dest_mint.to_account_info(),
            to: ctx.accounts.recipient_dest_ata.to_account_info(),
            authority: ctx.accounts.relayer.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
        );
        token_2022::transfer_checked(
            cpi_ctx,
            payment.dest_amount,
            ctx.accounts.dest_mint.decimals,
        )?;

        let now = Clock::get()?.unix_timestamp;
        payment.status = PaymentStatus::Settled;
        payment.settled_at = now;

        emit!(PaymentSettled {
            payment: ctx.accounts.payment.key(),
            source_amount: payment.source_amount,
            dest_amount: payment.dest_amount,
            fx_rate: payment.fx_rate,
            settled_at: now,
        });

        msg!("Payment settled");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct UpdateOracle<'info> {
    #[account(
        init_if_needed,
        payer = authority,
        space = 8 + OraclePrice::INIT_SPACE,
        seeds = [b"oracle", source_mint.key().as_ref(), dest_mint.key().as_ref()],
        bump,
    )]
    pub oracle_price: Account<'info, OraclePrice>,
    /// CHECK: Source mint
    pub source_mint: UncheckedAccount<'info>,
    /// CHECK: Dest mint
    pub dest_mint: UncheckedAccount<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(amount: u64, travel_rule_hash: [u8; 32], nonce: u64)]
pub struct CreatePayment<'info> {
    #[account(
        init,
        payer = sender,
        space = 8 + Payment::INIT_SPACE,
        seeds = [b"payment", sender.key().as_ref(), nonce.to_le_bytes().as_ref()],
        bump,
    )]
    pub payment: Account<'info, Payment>,
    #[account(
        seeds = [b"oracle", source_mint.key().as_ref(), dest_mint.key().as_ref()],
        bump = oracle_price.bump,
    )]
    pub oracle_price: Account<'info, OraclePrice>,
    /// CHECK: Source mint
    pub source_mint: UncheckedAccount<'info>,
    /// CHECK: Dest mint
    pub dest_mint: UncheckedAccount<'info>,
    /// CHECK: Recipient
    pub recipient: UncheckedAccount<'info>,
    #[account(mut)]
    pub sender: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteSettlement<'info> {
    #[account(
        mut,
        seeds = [b"payment", payment.sender.as_ref(), payment.nonce.to_le_bytes().as_ref()],
        bump = payment.bump,
    )]
    pub payment: Account<'info, Payment>,
    pub source_mint: InterfaceAccount<'info, Mint>,
    pub dest_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub sender_source_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub protocol_source_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub protocol_dest_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub recipient_dest_ata: InterfaceAccount<'info, TokenAccount>,
    pub relayer: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
}
