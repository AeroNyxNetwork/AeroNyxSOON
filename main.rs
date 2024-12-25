use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{CloseAccount, Mint, Token, TokenAccount, Transfer};
use solana_program::hash::hash;
use std::str::FromStr;

declare_id!("AzqFSRjxR59LUdZcJxxmFauZhQSpxMFcmCHaKVXAEMDG");

// Constants: Using static constants to improve performance and maintainability
pub const INFO_SEED: &[u8] = b"server";
pub const MAIN_SEED: &[u8] = b"main";
pub const SPECIFIED_MINT: &str = "BPtPUxkZc1BR1uEDMUkheABh9N94PUbnXvmXRdCLECBW";
pub const DELEGATE_MINIMUM_STAKE: u64 = 500 * 1_000_000_000;
pub const MINIMUM_STAKE: u64 = 1000 * 1_000_000_000;
pub const MAXIMUM_STAKE: u64 = 10000 * 1_000_000_000;
pub const VERSION: u8 = 1;

#[program]
mod staking_contract {
    use super::*;

    pub fn initialize_main(ctx: Context<InitializeMain>) -> Result<()> {
        let main_account = &mut ctx.accounts.main_account;
        require!(!main_account.initialized, CustomError::AlreadyInitialized);
        main_account.initialized = true;

        emit!(MainAccountInitialized {
            admin: ctx.accounts.owner.key(),
        });

        Ok(())
    }

    pub fn add_server(
        ctx: Context<AddServer>,
        serverkey: Vec<u8>,
        server_name: String,
        amount: u64,
    ) -> Result<()> {
        // Validate input parameters
        if server_name.len() > 32 {
            return Err(CustomError::NameTooLong.into());
        }

        if serverkey.len() > 65 {
            return Err(ProgramError::InvalidArgument.into()); // Return error for invalid data length
        }

        // Safe mathematical operations
        let amount_in_minimum_units = amount
            .checked_mul(1_000_000_000)
            .ok_or(CustomError::NumberOverflow)?;

        if amount_in_minimum_units < MINIMUM_STAKE || amount_in_minimum_units > MAXIMUM_STAKE {
            return Err(CustomError::MoreThan1000FewerThan10000.into());
        }

        let main_account = &mut ctx.accounts.main_account;
        let info_account = &mut ctx.accounts.info_account;

        // If it's a new account, increase total users and set owner
        if !info_account.initialized {
            main_account.total_users += 1;
            info_account.owner = ctx.accounts.owner.key(); // Set to caller's public key
            info_account.name = server_name.clone(); // Store name
            info_account.serverkey = serverkey.clone();
            info_account.initialized = true; // Mark account as initialized
        } else {
            require!(
                info_account.owner == ctx.accounts.owner.key(),
                CustomError::InfoAlreadyInitialized
            );
        }

        // Transfer xxx tokens to PDA's TokenAccount
        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.sender_token_account.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            amount_in_minimum_units,
        )?;

        info_account.stake += amount_in_minimum_units;
        info_account.total += amount_in_minimum_units;
        main_account.total_stake += amount_in_minimum_units;

        // Record event
        emit!(ServerAdded {
            owner: ctx.accounts.owner.key(),
            name: server_name,
            amount: amount_in_minimum_units,
            serverkey: serverkey,
        });

        Ok(())
    }

    // Update server name
    pub fn update_server(ctx: Context<UpdateServer>, new_name: String) -> Result<()> {
        let info_account = &mut ctx.accounts.info_account;

        info_account.name = new_name.clone();

        emit!(ServerUpdated {
            owner: ctx.accounts.owner.key(),
            name: new_name,
            amount: info_account.stake,
            serverkey: (*info_account.serverkey.clone()).to_vec(),
        });

        Ok(())
    }

    // Remove node
    pub fn remove_server(ctx: Context<RemoveServer>) -> Result<()> {
        let main_account = &mut ctx.accounts.main_account;
        let owner = ctx.accounts.owner.key();

        let seeds = &[
            INFO_SEED,
            owner.as_ref(),
            &hash(ctx.accounts.info_account.serverkey.as_ref()).to_bytes(),
            &[ctx.bumps.info_account], // Use vault's seeds and bump
        ];

        anchor_spl::token::close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account: ctx.accounts.vault.to_account_info(),
                destination: ctx.accounts.owner.to_account_info(),
                authority: ctx.accounts.info_account.to_account_info(),
            },
            &[&seeds[..]], // PDA's seeds for signature
        ))?;

        main_account.total_users -= 1;

        emit!(ServerRemoved {
            owner,
            name: ctx.accounts.info_account.name.clone(),
            serverkey: ctx.accounts.info_account.serverkey.clone(),
        });
        Ok(())
    }

    pub fn d_remove(ctx: Context<RemoveDelegatedAccount>) -> Result<()> {
        let main_account = &mut ctx.accounts.main_account;
        let info_account = &mut ctx.accounts.info_account;
        let owner = ctx.accounts.owner.key();

        let binding = info_account.key();

        let seeds = &[
            INFO_SEED,
            owner.as_ref(),
            binding.as_ref(),
            &[ctx.bumps.delegated_account], // Use vault's seeds and bump
        ];

        anchor_spl::token::close_account(CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            CloseAccount {
                account: ctx.accounts.vault.to_account_info(),
                destination: ctx.accounts.owner.to_account_info(),
                authority: ctx.accounts.delegated_account.to_account_info(),
            },
            &[&seeds[..]], // PDA's seeds for signature
        ))?;

        main_account.total_users -= 1;
        info_account.total_delegators -= 1;

        emit!(DelegatedRemoved {
            owner,
            delegator: info_account.key(),
        });
        Ok(())
    }

    // Deposit stake amount
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        let main_account = &mut ctx.accounts.main_account;
        let info_account = &mut ctx.accounts.info_account;

        // require!(amount > 0, CustomError::InsufficientFunds);

        // Safe mathematical operations
        let amount_in_minimum_units = amount
            .checked_mul(1_000_000_000)
            .ok_or(CustomError::NumberOverflow)?;

        // Check if it exceeds the maximum stake limit
        require!(
            info_account.stake + amount_in_minimum_units <= MAXIMUM_STAKE,
            CustomError::ExceedsMaxStakeLimit
        );

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.sender_token_account.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            amount_in_minimum_units,
        )?;

        info_account.stake += amount_in_minimum_units;
        info_account.total += amount_in_minimum_units;
        main_account.total_stake += amount_in_minimum_units;

        // Record event
        emit!(TokenDeposited {
            owner: ctx.accounts.owner.key(),
            name: info_account.name.clone(),
            amount: info_account.stake,
        });

        Ok(())
    }

    pub fn d_deposit(ctx: Context<DelegatedDeposit>, amount: u64) -> Result<()> {
        let main_account = &mut ctx.accounts.main_account;
        let info_account = &mut ctx.accounts.info_account;
        let delegated_account = &mut ctx.accounts.delegated_account;

        if !delegated_account.initialized {
            main_account.total_users += 1;
            info_account.total_delegators += 1;
            delegated_account.owner = ctx.accounts.owner.key();
            delegated_account.delegator = info_account.key();
            delegated_account.initialized = true; // Mark account as initialized
        } else {
            require!(
                delegated_account.owner == ctx.accounts.owner.key(),
                CustomError::DelegateAlreadyInitialized
            );
        }

        // Safe mathematical operations
        let amount_in_minimum_units = amount
            .checked_mul(1_000_000_000)
            .ok_or(CustomError::NumberOverflow)?;

        if amount_in_minimum_units < DELEGATE_MINIMUM_STAKE
            || delegated_account.stake + amount_in_minimum_units > MAXIMUM_STAKE
        {
            return Err(CustomError::DelegateExceedsMaxStakeLimit.into());
        }

        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.sender_token_account.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            amount_in_minimum_units,
        )?;

        delegated_account.stake += amount_in_minimum_units;
        info_account.total += amount_in_minimum_units;
        main_account.total_stake += amount_in_minimum_units;

        // Record event
        emit!(TokenDelegatedDeposited {
            owner: ctx.accounts.owner.key(),
            delegator: info_account.key(),
            delegator_owner: info_account.owner.key(),
            amount: info_account.stake,
        });

        Ok(())
    }

    // Withdraw stake amount
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let main_account = &mut ctx.accounts.main_account;
        let info_account = &mut ctx.accounts.info_account;
        let owner = ctx.accounts.owner.key();

        let amount_in_minimum_units = amount * 1_000_000_000; // Convert amount to minimum units

        require!(
            amount_in_minimum_units <= info_account.stake,
            CustomError::InsufficientFunds
        );

        let serverkey = &info_account.serverkey;

        // Transfer xxx tokens from PDA TokenAccount to user's TokenAccount
        let seeds = &[
            INFO_SEED,
            owner.as_ref(),
            &hash(serverkey.as_ref()).to_bytes(),
            &[ctx.bumps.info_account], // Use vault's seeds and bump
        ];

        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.receipt_token_account.to_account_info(),
                    authority: ctx.accounts.info_account.to_account_info(), // Use vault as authority
                },
                &[&seeds[..]], // PDA's seeds
            ),
            amount_in_minimum_units,
        )?;

        ctx.accounts.info_account.stake -= amount_in_minimum_units;
        ctx.accounts.info_account.total -= amount_in_minimum_units;
        main_account.total_stake -= amount_in_minimum_units;

        // Record event
        emit!(TokenWithdrawn {
            owner: ctx.accounts.owner.key(),
            name: ctx.accounts.info_account.name.clone(),
            amount: ctx.accounts.info_account.stake,
        });

        Ok(())
    }

    pub fn d_withdraw(ctx: Context<DelegatedWithdraw>, amount: u64) -> Result<()> {
        let main_account = &mut ctx.accounts.main_account;
        let info_account = &mut ctx.accounts.info_account;
        let delegated_account = &mut ctx.accounts.delegated_account;
        let owner = ctx.accounts.owner.key();

        let amount_in_minimum_units = amount * 1_000_000_000; // Convert amount to minimum units

        require!(
            amount_in_minimum_units <= delegated_account.stake,
            CustomError::InsufficientFunds
        );

        let binding = info_account.key();

        let seeds = &[
            INFO_SEED,
            owner.as_ref(),
            binding.as_ref(),
            &[ctx.bumps.delegated_account], // Use vault's seeds and bump
        ];

        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.receipt_token_account.to_account_info(),
                    authority: delegated_account.to_account_info(),
                },
                &[&seeds[..]],
            ),
            amount_in_minimum_units,
        )?;

        info_account.total -= amount_in_minimum_units;
        delegated_account.stake -= amount_in_minimum_units;
        main_account.total_stake -= amount_in_minimum_units;

        // Record event
        emit!(DelegatedTokenWithdrawn {
            owner: owner.key(),
            delegator: info_account.key(),
            delegator_owner: info_account.owner.key(),
            amount: delegated_account.stake,
        });

        Ok(())
    }

}

#[derive(Accounts)]
pub struct InitializeMain<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + 8 + 4 +1, 
        seeds = [MAIN_SEED], 
        bump
    )]
    pub main_account: Account<'info, MainAccount>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(serverkey: Vec<u8>)]
pub struct AddServer<'info> {
    #[account(mut)]
    pub main_account: Account<'info, MainAccount>,

    // PDA account for storing data
    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + 1 + 32 + 8 + 4 + 32 + 69,
        seeds = [
            INFO_SEED,        // seed prefix
            owner.key().as_ref(), // Use caller's public key as seed
            &hash(serverkey.as_ref()).to_bytes(),
        ],
        bump
    )]
    pub info_account: Account<'info, InfoAccount>, // PDA for storing name

    // Transfer account
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program,
    )]
    pub sender_token_account: Account<'info, TokenAccount>,

    // PDA account for staking in contract
    #[account(
        init_if_needed,  
        payer = owner,
        associated_token::mint = mint,         // Specified token type
        associated_token::authority = info_account,         // Manager (can be other account, here is PDA account)
        associated_token::token_program = token_program,
    )]
    pub vault: Account<'info, TokenAccount>,

    // Hardcoded specified token Mint address
    #[account(
        address = Pubkey::from_str(SPECIFIED_MINT).unwrap() @ CustomError::InvalidMint
    )]
    pub mint: Account<'info, Mint>, // Specified token mint address

    #[account(mut)]
    pub owner: Signer<'info>,

    // Token Program
    pub token_program: Program<'info, Token>,

    // Associated Token Program
    pub associated_token_program: Program<'info, AssociatedToken>,

    // System Program
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateServer<'info> {
    #[account(
        mut,
        has_one = owner
    )]
    pub info_account: Account<'info, InfoAccount>, // For updating name
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct RemoveServer<'info> {
    #[account(mut)]
    pub main_account: Account<'info, MainAccount>,

    #[account(
        mut,
        close = owner,
        has_one = owner,
        constraint = info_account.total == 0 @ CustomError::NonZeroBalance,
        seeds = [
            INFO_SEED,        // seed prefix
            owner.key().as_ref(), // Use caller's public key as seed
            &hash(info_account.serverkey.as_ref()).to_bytes(),
        ],
        bump,     
    )]
    pub info_account: Account<'info, InfoAccount>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = info_account,
        associated_token::token_program = token_program,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        address = Pubkey::from_str(SPECIFIED_MINT).unwrap() @ CustomError::InvalidMint
    )]
    pub mint: Account<'info, Mint>, // Hardcoded specified token

    #[account(mut)]
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>, // System Program
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub main_account: Account<'info, MainAccount>,

    #[account(
        mut,
        has_one = owner,
    )]
    pub info_account: Account<'info, InfoAccount>, // PDA for storing name

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = info_account,
        associated_token::token_program = token_program,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        address = Pubkey::from_str(SPECIFIED_MINT).unwrap() @ CustomError::InvalidMint
    )]
    pub mint: Account<'info, Mint>,

    // Transfer account
    #[account(
        mut,
        constraint = sender_token_account.mint == mint.key() @ CustomError::InvalidMint,  
    )]
    pub sender_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DelegatedDeposit<'info> {
    #[account(mut)]
    pub main_account: Account<'info, MainAccount>,

    #[account(mut)]
    pub info_account: Account<'info, InfoAccount>,

    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + 1 + 32 + 32 + 8,
        seeds = [
            INFO_SEED,
            owner.key().as_ref(),
            info_account.key().as_ref(),
        ],
        bump
    )]
    pub delegated_account: Account<'info, DelegatedAccount>, // PDA account for staking in contract

    #[account(
        init_if_needed,  
        payer = owner,
        associated_token::mint = mint,
        associated_token::authority = delegated_account,
        associated_token::token_program = token_program,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        address = Pubkey::from_str(SPECIFIED_MINT).unwrap() @ CustomError::InvalidMint
    )]
    pub mint: Account<'info, Mint>,

    // Transfer account
    #[account(
        mut,
        constraint = sender_token_account.mint == mint.key() @ CustomError::InvalidMint,  
    )]
    pub sender_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub main_account: Account<'info, MainAccount>,

    #[account(
        mut,
        has_one = owner,
        seeds = [
            INFO_SEED,        // seed prefix
            owner.key().as_ref(), // Use caller's public key as seed
            &hash(info_account.serverkey.as_ref()).to_bytes(),
        ],
        bump
    )]
    pub info_account: Account<'info, InfoAccount>, // PDA for storing name
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = info_account,
        associated_token::token_program = token_program,
    )]
    pub vault: Account<'info, TokenAccount>,

    // Here, if there's no related ata account, the contract automatically creates or updates the account to accept tokens. The address of the ata account is easy to derive using @solana/spl-token's getAssociatedTokenAddress
    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program,
    )]
    pub receipt_token_account: Account<'info, TokenAccount>,

    #[account(
        address = Pubkey::from_str(SPECIFIED_MINT).unwrap() @ CustomError::InvalidMint
    )]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DelegatedWithdraw<'info> {
    #[account(mut)]
    pub main_account: Account<'info, MainAccount>,

    #[account(mut)]
    pub info_account: Account<'info, InfoAccount>,

    #[account(
        mut,
        has_one = owner,
        seeds = [
            INFO_SEED,
            owner.key().as_ref(),
            info_account.key().as_ref(),
        ],
        bump
    )]
    pub delegated_account: Account<'info, DelegatedAccount>, // PDA account for staking in contract

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = delegated_account,
        associated_token::token_program = token_program,
    )]
    pub vault: Account<'info, TokenAccount>,

    // Here, if there's no related ata account, the contract automatically creates or updates the account to accept tokens. The address of the ata account is easy to derive using @solana/spl-token's getAssociatedTokenAddress
    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program,
    )]
    pub receipt_token_account: Account<'info, TokenAccount>,

    #[account(
        address = Pubkey::from_str(SPECIFIED_MINT).unwrap() @ CustomError::InvalidMint
    )]
    pub mint: Account<'info, Mint>,

    #[account(mut)]
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RemoveDelegatedAccount<'info> {
    #[account(mut)]
    pub main_account: Account<'info, MainAccount>,
    #[account(mut)]
    pub info_account: Account<'info, InfoAccount>,

    #[account(
        mut,
        close = owner,
        has_one = owner,
        constraint = delegated_account.stake == 0 @ CustomError::NonZeroBalance,  // Can only close account when stake is 0
        seeds = [
            INFO_SEED,        // seed prefix
            owner.key().as_ref(), // Use caller's public key as seed
            info_account.key().as_ref(),
        ],
        bump,     
    )]
    pub delegated_account: Account<'info, DelegatedAccount>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = delegated_account,
        associated_token::token_program = token_program,
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        address = Pubkey::from_str(SPECIFIED_MINT).unwrap() @ CustomError::InvalidMint
    )]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct MainAccount {
    pub total_stake: u64,
    pub total_users: u32,
    pub initialized: bool,
}

#[account]
pub struct InfoAccount {
    pub initialized: bool,
    pub owner: Pubkey,
    pub stake: u64,
    pub total: u64,
    pub total_delegators: u32,
    pub name: String,
    pub serverkey: Vec<u8>,
}

#[account]
pub struct DelegatedAccount {
    pub initialized: bool,
    pub delegator: Pubkey,
    pub owner: Pubkey,
    pub stake: u64,
}

#[event]
pub struct MainAccountInitialized {
    pub admin: Pubkey,
}

#[event]
pub struct ServerAdded {
    #[index]
    pub owner: Pubkey,
    pub name: String,
    pub amount: u64,
    pub serverkey: Vec<u8>,
}

#[event]
pub struct ServerUpdated {
    #[index]
    pub owner: Pubkey,
    pub name: String,
    pub amount: u64,
    pub serverkey: Vec<u8>,
}

#[event]
pub struct ServerRemoved {
    #[index]
    pub owner: Pubkey,
    pub name: String,
    pub serverkey: Vec<u8>,
}

#[event]
pub struct DelegatedRemoved {
    #[index]
    pub owner: Pubkey,
    pub delegator: Pubkey,
}

#[event]
pub struct TokenDeposited {
    #[index]
    pub owner: Pubkey,
    pub name: String,
    pub amount: u64,
}

#[event]
pub struct TokenDelegatedDeposited {
    #[index]
    pub owner: Pubkey,
    pub delegator: Pubkey,
    pub delegator_owner: Pubkey,
    pub amount: u64,
}

#[event]
pub struct TokenWithdrawn {
    #[index]
    pub owner: Pubkey,
    pub name: String,
    pub amount: u64,
}

#[event]
pub struct DelegatedTokenWithdrawn {
    #[index]
    pub owner: Pubkey,
    pub delegator: Pubkey,
    pub delegator_owner: Pubkey,
    pub amount: u64,
}

#[error_code]
pub enum CustomError {
    #[msg("Already initialized.")]
    AlreadyInitialized,
    #[msg("Account already initialized. Cannot change owner.")]
    DepositAlreadyInitialized,
    #[msg("Server has already created.")]
    InfoAlreadyInitialized,
    #[msg("Account has already created.")]
    DelegateAlreadyInitialized,
    #[msg("Insufficient funds.")]
    InsufficientFunds,
    #[msg("The user stake account has a non-zero balance.")]
    NonZeroBalance,
    #[msg("Only the owner can do this action")]
    Unauthorized,
    #[msg("The provided mint does not match the specified mint.")]
    InvalidMint,
    #[msg("Create a server with more than 1,000  and fewer than 10,000 tokens.")]
    MoreThan1000FewerThan10000,
    #[msg("Create a delegated account with more than 500 tokens and the total stake cannot exceed 10,000 tokens.")]
    DelegateExceedsMaxStakeLimit,
    #[msg("The specified stake account was not found.")]
    StakeAccountNotFound,
    #[msg("The total stake cannot exceed 10,000.")]
    ExceedsMaxStakeLimit,
    #[msg("Name must not exceed 64 characters")]
    NameTooLong,
    #[msg("Number overflow")]
    NumberOverflow,
    #[msg("Server public should less than 66 bytes")]
    InvalidArgument,
    #[msg("Vault is not empty. Transfer or burn tokens before closing.")]
    VaultNotEmpty,
    #[msg("Only owner can update server name.")]
    OnlyOwnwer,
}
