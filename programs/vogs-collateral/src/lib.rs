use anchor_lang::prelude::*;
use anchor_spl::token_2022::{self, Token2022, TransferChecked};
use anchor_spl::token_interface::{Mint, TokenAccount};

declare_id!("VCo11at11111111111111111111111111111111111");

pub mod state;
pub mod errors;
pub mod events;

use errors::CollateralError;
use events::{MarginCallAlert, PositionLiquidated};
use state::{CollateralPosition, PositionStatus};

#[program]
pub mod vogs_collateral {
    use super::*;

    pub fn create_position(
        ctx: Context<CreatePosition>,
        collateral_amount: u64,
        max_ltv_bps: u16,
        margin_call_ltv_bps: u16,
        liquidation_ltv_bps: u16,
        nonce: u64,
    ) -> Result<()> {
        // Transfer collateral to escrow
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.owner_collateral_ata.to_account_info(),
            mint: ctx.accounts.collateral_mint.to_account_info(),
            to: ctx.accounts.collateral_escrow.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token_2022::transfer_checked(cpi_ctx, collateral_amount, ctx.accounts.collateral_mint.decimals)?;

        let position = &mut ctx.accounts.position;
        position.owner = ctx.accounts.owner.key();
        position.collateral_mint = ctx.accounts.collateral_mint.key();
        position.credit_mint = ctx.accounts.credit_mint.key();
        position.collateral_escrow = ctx.accounts.collateral_escrow.key();
        position.pyth_price_account = ctx.accounts.pyth_price_account.key();
        position.collateral_amount = collateral_amount;
        position.collateral_decimals = ctx.accounts.collateral_mint.decimals;
        position.credit_drawn = 0;
        position.max_ltv_bps = max_ltv_bps;
        position.margin_call_ltv_bps = margin_call_ltv_bps;
        position.liquidation_ltv_bps = liquidation_ltv_bps;
        position.status = PositionStatus::Active;
        position.created_at = Clock::get()?.unix_timestamp;
        position.nonce = nonce;
        position.bump = ctx.bumps.position;

        msg!("Collateral position created: {} tokens pledged", collateral_amount);
        Ok(())
    }

    pub fn draw_credit(ctx: Context<DrawCredit>, amount: u64) -> Result<()> {
        let position = &mut ctx.accounts.position;
        require!(position.status == PositionStatus::Active, CollateralError::PositionNotActive);

        // Read price from Pyth oracle account (simplified: read price as u64 from account data)
        let price_data = ctx.accounts.pyth_price_account.try_borrow_data()?;
        let collateral_price = if price_data.len() >= 8 {
            u64::from_le_bytes(price_data[0..8].try_into().unwrap())
        } else {
            return Err(CollateralError::OracleError.into());
        };

        // Compute LTV: (credit_drawn + amount) / (collateral_amount * price)
        let total_credit = position.credit_drawn.checked_add(amount).unwrap();
        let collateral_value = (position.collateral_amount as u128)
            .checked_mul(collateral_price as u128)
            .unwrap()
            .checked_div(10u128.pow(position.collateral_decimals as u32))
            .unwrap();

        let ltv_bps = if collateral_value > 0 {
            ((total_credit as u128) * 10_000 / collateral_value) as u16
        } else {
            return Err(CollateralError::LtvExceeded.into());
        };

        require!(ltv_bps <= position.max_ltv_bps, CollateralError::LtvExceeded);

        position.credit_drawn = total_credit;

        // Emit margin call alert if above threshold
        if ltv_bps > position.margin_call_ltv_bps {
            emit!(MarginCallAlert {
                position: ctx.accounts.position.key(),
                current_ltv_bps: ltv_bps,
                margin_call_ltv_bps: position.margin_call_ltv_bps,
                collateral_value: collateral_value as u64,
                credit_drawn: total_credit,
            });
        }

        msg!("Drew {} credit, LTV: {} bps", amount, ltv_bps);
        Ok(())
    }

    pub fn repay_credit(ctx: Context<RepayCredit>, amount: u64) -> Result<()> {
        let position = &mut ctx.accounts.position;

        // Transfer stablecoins from user to treasury
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.owner_credit_ata.to_account_info(),
            mint: ctx.accounts.credit_mint.to_account_info(),
            to: ctx.accounts.treasury_credit_ata.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token_2022::transfer_checked(cpi_ctx, amount, ctx.accounts.credit_mint.decimals)?;

        position.credit_drawn = position.credit_drawn.saturating_sub(amount);

        msg!("Repaid {} credit, remaining: {}", amount, position.credit_drawn);
        Ok(())
    }

    pub fn withdraw_collateral(ctx: Context<WithdrawCollateral>, amount: u64) -> Result<()> {
        let position = &mut ctx.accounts.position;

        // Read price
        let price_data = ctx.accounts.pyth_price_account.try_borrow_data()?;
        let collateral_price = if price_data.len() >= 8 {
            u64::from_le_bytes(price_data[0..8].try_into().unwrap())
        } else {
            return Err(CollateralError::OracleError.into());
        };

        // Check LTV after withdrawal
        let remaining_collateral = position.collateral_amount.checked_sub(amount).unwrap();
        let collateral_value = (remaining_collateral as u128)
            .checked_mul(collateral_price as u128)
            .unwrap()
            .checked_div(10u128.pow(position.collateral_decimals as u32))
            .unwrap();

        if position.credit_drawn > 0 && collateral_value > 0 {
            let ltv_bps = ((position.credit_drawn as u128) * 10_000 / collateral_value) as u16;
            require!(ltv_bps <= position.max_ltv_bps, CollateralError::WithdrawalWouldBreachLtv);
        }

        // Transfer collateral back to owner
        let owner_key = position.owner;
        let collateral_mint_key = position.collateral_mint;
        let nonce_bytes = position.nonce.to_le_bytes();
        let seeds = &[
            b"position",
            owner_key.as_ref(),
            collateral_mint_key.as_ref(),
            nonce_bytes.as_ref(),
            &[position.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = TransferChecked {
            from: ctx.accounts.collateral_escrow.to_account_info(),
            mint: ctx.accounts.collateral_mint.to_account_info(),
            to: ctx.accounts.owner_collateral_ata.to_account_info(),
            authority: ctx.accounts.position.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token_2022::transfer_checked(cpi_ctx, amount, ctx.accounts.collateral_mint.decimals)?;

        position.collateral_amount = remaining_collateral;

        msg!("Withdrew {} collateral", amount);
        Ok(())
    }

    pub fn liquidate(ctx: Context<Liquidate>) -> Result<()> {
        let position = &mut ctx.accounts.position;
        require!(position.status == PositionStatus::Active, CollateralError::PositionNotActive);

        // Read price
        let price_data = ctx.accounts.pyth_price_account.try_borrow_data()?;
        let collateral_price = if price_data.len() >= 8 {
            u64::from_le_bytes(price_data[0..8].try_into().unwrap())
        } else {
            return Err(CollateralError::OracleError.into());
        };

        let collateral_value = (position.collateral_amount as u128)
            .checked_mul(collateral_price as u128)
            .unwrap()
            .checked_div(10u128.pow(position.collateral_decimals as u32))
            .unwrap();

        let ltv_bps = if collateral_value > 0 {
            ((position.credit_drawn as u128) * 10_000 / collateral_value) as u16
        } else {
            10_001 // Force liquidation if collateral is worthless
        };

        require!(
            ltv_bps > position.liquidation_ltv_bps,
            CollateralError::PositionNotLiquidatable
        );

        // Transfer all collateral to liquidator
        let owner_key = position.owner;
        let collateral_mint_key = position.collateral_mint;
        let nonce_bytes = position.nonce.to_le_bytes();
        let seeds = &[
            b"position",
            owner_key.as_ref(),
            collateral_mint_key.as_ref(),
            nonce_bytes.as_ref(),
            &[position.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = TransferChecked {
            from: ctx.accounts.collateral_escrow.to_account_info(),
            mint: ctx.accounts.collateral_mint.to_account_info(),
            to: ctx.accounts.liquidator_collateral_ata.to_account_info(),
            authority: ctx.accounts.position.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token_2022::transfer_checked(
            cpi_ctx,
            position.collateral_amount,
            ctx.accounts.collateral_mint.decimals,
        )?;

        position.status = PositionStatus::Liquidated;

        emit!(PositionLiquidated {
            position: ctx.accounts.position.key(),
            liquidator: ctx.accounts.liquidator.key(),
            collateral_amount: position.collateral_amount,
            credit_drawn: position.credit_drawn,
            ltv_bps,
        });

        msg!("Position liquidated at {} bps LTV", ltv_bps);
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(collateral_amount: u64, max_ltv_bps: u16, margin_call_ltv_bps: u16, liquidation_ltv_bps: u16, nonce: u64)]
pub struct CreatePosition<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + CollateralPosition::INIT_SPACE,
        seeds = [b"position", owner.key().as_ref(), collateral_mint.key().as_ref(), nonce.to_le_bytes().as_ref()],
        bump,
    )]
    pub position: Account<'info, CollateralPosition>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    pub credit_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub collateral_escrow: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub owner_collateral_ata: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: Pyth price account
    pub pyth_price_account: UncheckedAccount<'info>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DrawCredit<'info> {
    #[account(
        mut,
        seeds = [b"position", position.owner.as_ref(), position.collateral_mint.as_ref(), position.nonce.to_le_bytes().as_ref()],
        bump = position.bump,
        has_one = owner,
    )]
    pub position: Account<'info, CollateralPosition>,
    /// CHECK: Pyth price account
    pub pyth_price_account: UncheckedAccount<'info>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct RepayCredit<'info> {
    #[account(
        mut,
        seeds = [b"position", position.owner.as_ref(), position.collateral_mint.as_ref(), position.nonce.to_le_bytes().as_ref()],
        bump = position.bump,
        has_one = owner,
    )]
    pub position: Account<'info, CollateralPosition>,
    pub credit_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub owner_credit_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub treasury_credit_ata: InterfaceAccount<'info, TokenAccount>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct WithdrawCollateral<'info> {
    #[account(
        mut,
        seeds = [b"position", position.owner.as_ref(), position.collateral_mint.as_ref(), position.nonce.to_le_bytes().as_ref()],
        bump = position.bump,
        has_one = owner,
    )]
    pub position: Account<'info, CollateralPosition>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub collateral_escrow: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub owner_collateral_ata: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: Pyth price account
    pub pyth_price_account: UncheckedAccount<'info>,
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct Liquidate<'info> {
    #[account(
        mut,
        seeds = [b"position", position.owner.as_ref(), position.collateral_mint.as_ref(), position.nonce.to_le_bytes().as_ref()],
        bump = position.bump,
    )]
    pub position: Account<'info, CollateralPosition>,
    pub collateral_mint: InterfaceAccount<'info, Mint>,
    #[account(mut)]
    pub collateral_escrow: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub liquidator_collateral_ata: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: Pyth price account
    pub pyth_price_account: UncheckedAccount<'info>,
    pub liquidator: Signer<'info>,
    pub token_program: Program<'info, Token2022>,
}
