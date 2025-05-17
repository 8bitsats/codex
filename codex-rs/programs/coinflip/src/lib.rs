use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("<PROGRAM_ID>");

#[program]
pub mod coinflip {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let state = &mut ctx.accounts.state;
        state.owner = ctx.accounts.owner.key();
        state.state_bump = *ctx.bumps.get("state").unwrap();
        state.vault_bump = *ctx.bumps.get("vault").unwrap();
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        let state = &ctx.accounts.state;
        require_keys_eq!(
            ctx.accounts.owner.key(),
            state.owner,
            CustomError::Unauthorized
        );
        let cpi_accounts = Transfer {
            from: ctx.accounts.owner_token_account.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;
        Ok(())
    }

    pub fn flip(ctx: Context<Flip>, amount: u64, side: u8) -> Result<()> {
        let state = &ctx.accounts.state;
        require!(side < 2, CustomError::InvalidSide);
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        let clock = Clock::get()?;
        let mut data = clock.unix_timestamp.to_le_bytes().to_vec();
        data.extend_from_slice(ctx.accounts.user.key.as_ref());
        let result = hash(&data).0[0] % 2;
        if result == side {
            let payout = amount.checked_mul(2).ok_or(CustomError::Overflow)?;
            let vault_seeds = &[b"vault", &[state.vault_bump]];
            let signer = &[&vault_seeds[..]];
            let cpi_accounts = Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            };
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                signer,
            );
            token::transfer(cpi_ctx, payout)?;
        }
        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let state = &ctx.accounts.state;
        require_keys_eq!(
            ctx.accounts.owner.key(),
            state.owner,
            CustomError::Unauthorized
        );
        let vault_seeds = &[b"vault", &[state.vault_bump]];
        let signer = &[&vault_seeds[..]];
        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.owner_token_account.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        );
        token::transfer(cpi_ctx, amount)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, seeds = [b"state"], bump, payer = owner, space = 8 + State::LEN)]
    pub state: Account<'info, State>,
    #[account(init, seeds = [b"vault"], bump, payer = owner, token::mint = mint, token::authority = vault_authority)]
    pub vault: Account<'info, TokenAccount>,
    /// CHECK: PDA authority for vault
    #[account(seeds = [b"vault"], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub mint: Account<'info, anchor_spl::token::Mint>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut, seeds = [b"state"], bump = state.state_bump)]
    pub state: Account<'info, State>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut)]
    pub owner_token_account: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"vault"], bump = state.vault_bump)]
    pub vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Flip<'info> {
    #[account(seeds = [b"state"], bump = state.state_bump)]
    pub state: Account<'info, State>,
    /// CHECK: PDA authority for vault
    #[account(seeds = [b"vault"], bump = state.vault_bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut, seeds = [b"vault"], bump = state.vault_bump)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut, seeds = [b"state"], bump = state.state_bump)]
    pub state: Account<'info, State>,
    /// CHECK: PDA authority for vault
    #[account(seeds = [b"vault"], bump = state.vault_bump)]
    pub vault_authority: UncheckedAccount<'info>,
    #[account(mut, seeds = [b"vault"], bump = state.vault_bump)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut)]
    pub owner_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct State {
    pub owner: Pubkey,
    pub state_bump: u8,
    pub vault_bump: u8,
}

impl State {
    const LEN: usize = 32 + 1 + 1;
}

#[error_code]
pub enum CustomError {
    #[msg("Invalid side, must be 0 or 1")]
    InvalidSide,
    #[msg("Overflow occurred")]
    Overflow,
    #[msg("Unauthorized")]
    Unauthorized,
}
