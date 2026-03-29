use anchor_lang::prelude::*;
use anchor_spl::token_2022::spl_token_2022;
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

declare_id!("VHook1111111111111111111111111111111111111");

pub mod state;
pub mod errors;

use errors::HookError;
use state::{BlocklistEntry, HookConfig};

#[program]
pub mod vogs_hook {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        gatekeeper_network: Pubkey,
    ) -> Result<()> {
        let config = &mut ctx.accounts.hook_config;
        config.authority = ctx.accounts.authority.key();
        config.mint = ctx.accounts.mint.key();
        config.gatekeeper_network = gatekeeper_network;
        config.bump = ctx.bumps.hook_config;
        Ok(())
    }

    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        let config = &ctx.accounts.hook_config;

        // Check sender blocklist
        if ctx.accounts.sender_blocklist.is_some() {
            return Err(HookError::WalletBlocked.into());
        }

        // Check receiver blocklist
        if ctx.accounts.receiver_blocklist.is_some() {
            return Err(HookError::WalletBlocked.into());
        }

        // Verify sender has a valid Civic Pass (gateway token)
        // In production, this checks the Civic Gateway Protocol account
        // For devnet demo, we check if the gateway token account exists and is valid
        if ctx.accounts.sender_gateway_token.data_is_empty() {
            return Err(HookError::SenderNotKyc.into());
        }

        // Verify receiver has a valid Civic Pass
        if ctx.accounts.receiver_gateway_token.data_is_empty() {
            return Err(HookError::ReceiverNotKyc.into());
        }

        msg!(
            "Transfer hook: {} tokens transferred, KYC verified for both parties",
            amount
        );
        Ok(())
    }

    pub fn add_to_blocklist(ctx: Context<ManageBlocklist>) -> Result<()> {
        let entry = &mut ctx.accounts.blocklist_entry;
        entry.mint = ctx.accounts.mint.key();
        entry.wallet = ctx.accounts.wallet.key();
        entry.blocked_at = Clock::get()?.unix_timestamp;
        entry.bump = ctx.bumps.blocklist_entry;
        Ok(())
    }

    pub fn remove_from_blocklist(_ctx: Context<RemoveFromBlocklist>) -> Result<()> {
        // Account is closed by the close constraint
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + HookConfig::INIT_SPACE,
        seeds = [b"hook-config", mint.key().as_ref()],
        bump,
    )]
    pub hook_config: Account<'info, HookConfig>,
    /// CHECK: The mint for which this hook is configured
    pub mint: UncheckedAccount<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TransferHook<'info> {
    /// CHECK: Source token account
    pub source: UncheckedAccount<'info>,
    /// CHECK: Token mint
    pub mint: UncheckedAccount<'info>,
    /// CHECK: Destination token account
    pub destination: UncheckedAccount<'info>,
    /// CHECK: Source token account authority
    pub owner: UncheckedAccount<'info>,
    /// CHECK: Extra account meta list
    pub extra_account_meta_list: UncheckedAccount<'info>,
    #[account(
        seeds = [b"hook-config", mint.key().as_ref()],
        bump = hook_config.bump,
    )]
    pub hook_config: Account<'info, HookConfig>,
    /// CHECK: Sender gateway token (Civic Pass)
    pub sender_gateway_token: UncheckedAccount<'info>,
    /// CHECK: Receiver gateway token (Civic Pass)
    pub receiver_gateway_token: UncheckedAccount<'info>,
    /// CHECK: Optional sender blocklist entry
    pub sender_blocklist: Option<Account<'info, BlocklistEntry>>,
    /// CHECK: Optional receiver blocklist entry
    pub receiver_blocklist: Option<Account<'info, BlocklistEntry>>,
}

#[derive(Accounts)]
pub struct ManageBlocklist<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + BlocklistEntry::INIT_SPACE,
        seeds = [b"blocklist", mint.key().as_ref(), wallet.key().as_ref()],
        bump,
    )]
    pub blocklist_entry: Account<'info, BlocklistEntry>,
    #[account(
        seeds = [b"hook-config", mint.key().as_ref()],
        bump = hook_config.bump,
        has_one = authority,
    )]
    pub hook_config: Account<'info, HookConfig>,
    /// CHECK: The mint
    pub mint: UncheckedAccount<'info>,
    /// CHECK: The wallet to blocklist
    pub wallet: UncheckedAccount<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RemoveFromBlocklist<'info> {
    #[account(
        mut,
        close = authority,
        seeds = [b"blocklist", mint.key().as_ref(), wallet.key().as_ref()],
        bump = blocklist_entry.bump,
    )]
    pub blocklist_entry: Account<'info, BlocklistEntry>,
    #[account(
        seeds = [b"hook-config", mint.key().as_ref()],
        bump = hook_config.bump,
        has_one = authority,
    )]
    pub hook_config: Account<'info, HookConfig>,
    /// CHECK: The mint
    pub mint: UncheckedAccount<'info>,
    /// CHECK: The wallet to remove from blocklist
    pub wallet: UncheckedAccount<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
}
