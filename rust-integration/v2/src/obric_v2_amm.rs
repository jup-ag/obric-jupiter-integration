use anchor_lang::{declare_id, AccountDeserialize};
use anyhow::{anyhow, bail, Result};
use jupiter_amm_interface::{
    try_get_account_data, AccountMap, Amm, AmmContext, ClockRef, KeyedAccount, Quote, QuoteParams,
    Swap, SwapAndAccountMetas, SwapParams,
};
use obric_solana::state::{parse_price, SSTradingPair};
use solana_sdk::{instruction::AccountMeta, program_pack::Pack, pubkey::Pubkey};
use spl_token::state::{Account as TokenAccount, Mint};

declare_id!("obriQD1zbpyLz95G5n7nJe6a4DPjpFwa5XYPoNm113y");

#[derive(Clone)]
pub struct ObricV2Amm {
    pub key: Pubkey,
    pub state: SSTradingPair,
    pub current_x: u64,
    pub current_y: u64,
    pub x_decimals: u8,
    pub y_decimals: u8,
    pub clock_ref: ClockRef,
    pub x_price_publish_time: i64,
    pub y_price_publish_time: i64,
}

impl Amm for ObricV2Amm {
    fn from_keyed_account(keyed_account: &KeyedAccount, amm_context: &AmmContext) -> Result<Self> {
        let data = &mut &keyed_account.account.data.clone()[0..];
        let ss_trading_pair = SSTradingPair::try_deserialize(data)?;

        Ok(Self {
            key: keyed_account.key,
            state: ss_trading_pair,
            current_x: 0u64,
            current_y: 0u64,
            x_decimals: 0u8,
            y_decimals: 0u8,
            clock_ref: amm_context.clock_ref.clone(),
            x_price_publish_time: 0,
            y_price_publish_time: 0,
        })
    }

    fn label(&self) -> String {
        String::from("Obric V2")
    }

    fn program_id(&self) -> Pubkey {
        self::id()
    }

    fn key(&self) -> Pubkey {
        self.key
    }

    fn get_reserve_mints(&self) -> Vec<Pubkey> {
        [self.state.mint_x, self.state.mint_y].to_vec()
    }

    fn get_accounts_to_update(&self) -> Vec<Pubkey> {
        let mut accounts = vec![
            self.key(),
            self.state.reserve_x,
            self.state.reserve_y,
            self.state.x_price_feed_id,
            self.state.y_price_feed_id,
        ];

        if self.x_decimals == 0 && self.y_decimals == 0 {
            accounts.extend([self.state.mint_x, self.state.mint_y]);
        }

        accounts
    }

    fn has_dynamic_accounts(&self) -> bool {
        true
    }

    fn update(&mut self, account_map: &AccountMap) -> Result<()> {
        let trading_pair_account =
            SSTradingPair::try_deserialize(&mut try_get_account_data(account_map, &self.key())?)?;
        let reserve_x_token_account =
            TokenAccount::unpack(try_get_account_data(account_map, &self.state.reserve_x)?)?;
        let reserve_y_token_account =
            TokenAccount::unpack(try_get_account_data(account_map, &self.state.reserve_y)?)?;

        let price_x_data = &mut try_get_account_data(account_map, &self.state.x_price_feed_id)?;
        let price_y_data = &mut try_get_account_data(account_map, &self.state.y_price_feed_id)?;

        self.state = trading_pair_account;
        self.current_x = reserve_x_token_account.amount;
        self.current_y = reserve_y_token_account.amount;

        if self.current_x == 0 || self.current_y == 0 {
            return Ok(());
        }

        if self.x_decimals == 0 && self.y_decimals == 0 {
            let mint_x = Mint::unpack(try_get_account_data(account_map, &self.state.mint_x)?)?;
            let mint_y = &Mint::unpack(try_get_account_data(account_map, &self.state.mint_y)?)?;

            self.x_decimals = mint_x.decimals;
            self.y_decimals = mint_y.decimals;
        }

        let time = self
            .clock_ref
            .unix_timestamp
            .load(std::sync::atomic::Ordering::Relaxed);

        let (x_price, x_publish_time) = parse_price(
            price_x_data, 
            self.state.price_decimals, 
            time, 
            self.state.feed_max_age_x
        )?;

        let (y_price, y_publish_time) = parse_price(
            price_y_data, 
            self.state.price_decimals, 
            time, 
            self.state.feed_max_age_y
        )?;

        self.x_price_publish_time = x_publish_time;
        self.y_price_publish_time = y_publish_time;

        self.state.update_price(
            x_price,
            y_price,
            self.x_decimals,
            self.y_decimals,
        )?;

        Ok(())
    }

    fn quote(&self, quote_params: &QuoteParams) -> Result<Quote> {
        let time = self
            .clock_ref
            .unix_timestamp
            .load(std::sync::atomic::Ordering::Relaxed);

        let x_age = time
            .checked_sub(self.x_price_publish_time)
            .ok_or(anyhow!("overflow"))?;
        let y_age = time
            .checked_sub(self.y_price_publish_time)
            .ok_or(anyhow!("overflow"))?;
        if x_age > self.state.feed_max_age_x as i64 || y_age > self.state.feed_max_age_y as i64 {
            return Err(anyhow!("stale price feed"));
        }

        let (output_after_fee, fee_amount) =
            if quote_params.input_mint.eq(&self.state.mint_x) {
                self.state
                    .quote_x_to_y(quote_params.amount, self.current_x, self.current_y)?
            } else if quote_params.input_mint.eq(&self.state.mint_y) {
                self.state
                    .quote_y_to_x(quote_params.amount, self.current_x, self.current_y)?
            } else {
                bail!("Quote doesn't return");
            };

        Ok(Quote {
            out_amount: output_after_fee,
            fee_amount,
            fee_mint: quote_params.output_mint,
            ..Quote::default()
        })
    }

    fn clone_amm(&self) -> Box<dyn Amm + Send + Sync> {
        Box::new(self.clone())
    }

    fn get_swap_and_account_metas(&self, swap_params: &SwapParams) -> Result<SwapAndAccountMetas> {
        let (x_to_y, user_token_account_x, user_token_account_y) =
            if swap_params.source_mint.eq(&self.state.mint_x) {
                (
                    true,
                    swap_params.source_token_account,
                    swap_params.destination_token_account,
                )
            } else {
                (
                    false,
                    swap_params.destination_token_account,
                    swap_params.source_token_account,
                )
            };

        Ok(SwapAndAccountMetas {
            swap: Swap::Obric { x_to_y },
            account_metas: vec![
                AccountMeta::new_readonly(self::ID, false),
                AccountMeta::new(self.key(), false),
                AccountMeta::new_readonly(self.state.mint_x, false),
                AccountMeta::new_readonly(self.state.mint_y, false),
                AccountMeta::new(self.state.reserve_x, false),
                AccountMeta::new(self.state.reserve_y, false),
                AccountMeta::new(user_token_account_x, false),
                AccountMeta::new(user_token_account_y, false),
                AccountMeta::new(self.state.reference_oracle, false),
                AccountMeta::new_readonly(self.state.x_price_feed_id, false),
                AccountMeta::new_readonly(self.state.y_price_feed_id, false),
                AccountMeta::new_readonly(swap_params.token_transfer_authority, false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
        })
    }

    fn get_accounts_len(&self) -> usize {
        12
    }
}
