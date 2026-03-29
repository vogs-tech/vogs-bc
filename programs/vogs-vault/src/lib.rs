use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, TransferChecked};
use anchor_spl::token_interface::{Mint, TokenAccount};

declare_id!("VVau1t111111111111111111111111111111111111");

pub mod state;
pub mod errors;

use errors::VaultError;
use state::{DepositorRecord, ProtocolAllocation, Vault};

const MAX_ALLOCATIONS: usize = 5;

#[program]
pub mod vogs_vault {
    use super::*;

    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        liquidity_buffer_bps: u16,
    ) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.authority = ctx.accounts.authority.key();
        vault.institution = ctx.accounts.institution.key();
        vault.mint = ctx.accounts.mint.key();
        vault.token_account = ctx.accounts.vault_token_account.key();
        vault.total_deposits = 0;
        vault.liquidity_buffer_bps = liquidity_buffer_bps;
        vault.allocations = Vec::new();
        vault.bump = ctx.bumps.vault;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        // Transfer tokens from user to vault via Token-2022 (triggers Transfer Hook)
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.user_token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token_2022::transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)?;

        // Update depositor record
        let depositor = &mut ctx.accounts.depositor_record;
        depositor.vault = ctx.accounts.vault.key();
        depositor.depositor = ctx.accounts.user.key();
        depositor.balance = depositor.balance.checked_add(amount).unwrap();
        depositor.last_deposit_at = Clock::get()?.unix_timestamp;
        depositor.bump = ctx.bumps.depositor_record;

        // Update vault total
        let vault = &mut ctx.accounts.vault;
        vault.total_deposits = vault.total_deposits.checked_add(amount).unwrap();

        msg!("Deposited {} tokens into vault", amount);
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let depositor = &mut ctx.accounts.depositor_record;
        require!(depositor.balance >= amount, VaultError::InsufficientBalance);

        // Transfer tokens from vault to user (PDA signer)
        let vault = &ctx.accounts.vault;
        let institution_key = vault.institution;
        let mint_key = vault.mint;
        let seeds = &[
            b"vault",
            institution_key.as_ref(),
            mint_key.as_ref(),
            &[vault.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = TransferChecked {
            from: ctx.accounts.vault_token_account.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token_2022::transfer_checked(cpi_ctx, amount, ctx.accounts.mint.decimals)?;

        // Update balances
        depositor.balance = depositor.balance.checked_sub(amount).unwrap();
        let vault = &mut ctx.accounts.vault;
        vault.total_deposits = vault.total_deposits.checked_sub(amount).unwrap();

        msg!("Withdrew {} tokens from vault", amount);
        Ok(())
    }

    pub fn update_allocations(
        ctx: Context<UpdateAllocations>,
        allocations: Vec<ProtocolAllocation>,
    ) -> Result<()> {
        require!(allocations.len() <= MAX_ALLOCATIONS, VaultError::TooManyAllocations);

        let total_bps: u16 = allocations.iter().map(|a| a.target_bps).sum();
        require!(total_bps == 10_000, VaultError::InvalidAllocationSum);

        let vault = &mut ctx.accounts.vault;
        vault.allocations = allocations;

        msg!("Updated vault allocations");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Vault::INIT_SPACE,
        seeds = [b"vault", institution.key().as_ref(), mint.key().as_ref()],
        bump,
    )]
    pub vault: Account<'info, Vault>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = vault,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: Institution identifier
    pub institution: UncheckedAccount<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        mut,
        seeds = [b"vault", vault.institution.as_ref(), vault.mint.as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + DepositorRecord::INIT_SPACE,
        seeds = [b"depositor", vault.key().as_ref(), user.key().as_ref()],
        bump,
    )]
    pub depositor_record: Account<'info, DepositorRecord>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = vault,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = user,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
        mut,
        seeds = [b"vault", vault.institution.as_ref(), vault.mint.as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,
    #[account(
        mut,
        seeds = [b"depositor", vault.key().as_ref(), user.key().as_ref()],
        bump = depositor_record.bump,
        has_one = depositor @ VaultError::Unauthorized,
    )]
    pub depositor_record: Account<'info, DepositorRecord>,
    pub mint: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = vault,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = mint,
        token::authority = user,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct UpdateAllocations<'info> {
    #[account(
        mut,
        seeds = [b"vault", vault.institution.as_ref(), vault.mint.as_ref()],
        bump = vault.bump,
        has_one = authority,
    )]
    pub vault: Account<'info, Vault>,
    pub authority: Signer<'info>,
}
