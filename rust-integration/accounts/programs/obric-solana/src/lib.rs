pub mod consts;
pub mod errors;
pub mod state;

use crate::errors::ObricError;
use crate::state::SSTradingPair;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("obriQD1zbpyLz95G5n7nJe6a4DPjpFwa5XYPoNm113y");

#[program]
pub mod obric_solana {
    use super::*;
    use crate::Swap;

    pub fn swap(
        _ctx: Context<Swap>,
        _is_x_to_y: bool,
        _input_amt: u64,
        _min_output_amt: u64,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(is_x_to_y: bool)]
pub struct Swap<'info> {
    #[account(
        mut,
        seeds = [consts::TRADING_PAIR_SEED.as_bytes(), mint_x.key().as_ref(), mint_y.key().as_ref()],
        bump = trading_pair.bump
    )]
    pub trading_pair: Box<Account<'info, SSTradingPair>>,

    pub mint_x: Box<Account<'info, Mint>>,

    pub mint_y: Box<Account<'info, Mint>>,

    #[account(mut, address = trading_pair.reserve_x)]
    pub reserve_x: Box<Account<'info, TokenAccount>>,

    #[account(mut, address = trading_pair.reserve_y)]
    pub reserve_y: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = user_token_account_x.owner == user.key(),
        constraint = user_token_account_x.mint == mint_x.key(),
    )]
    pub user_token_account_x: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = user_token_account_y.owner == user.key(),
        constraint = user_token_account_y.mint == mint_y.key(),
    )]
    pub user_token_account_y: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = (is_x_to_y && protocol_fee.key() == trading_pair.protocol_fee_y) ||
        (!is_x_to_y && protocol_fee.key() == trading_pair.protocol_fee_x)
    )]
    pub protocol_fee: Box<Account<'info, TokenAccount>>,

    #[account(address = trading_pair.x_price_feed_id @ ObricError::InvalidPriceAccount)]
    pub x_price_feed: AccountInfo<'info>,

    #[account(address = trading_pair.y_price_feed_id @ ObricError::InvalidPriceAccount)]
    pub y_price_feed: AccountInfo<'info>,

    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}
