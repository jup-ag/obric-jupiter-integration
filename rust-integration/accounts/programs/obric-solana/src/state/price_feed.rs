use anchor_lang::prelude::*;
use pyth_sdk::{Price, UnixTimestamp};
use pyth_sdk_solana::state::{load_price_account, SolanaPriceAccount};

use crate::errors::ObricError;

#[derive(Clone, Debug)]
pub struct PriceFeed(pub pyth_sdk::PriceFeed);

impl PriceFeed {
    pub fn get_timestamp(&self) -> UnixTimestamp {
        self.0.get_price_unchecked().publish_time
    }

    pub fn price_normalized(&self, current_time: UnixTimestamp, age: u64) -> Result<Price> {
        let p = self
            .0
            .get_price_no_older_than(current_time, age)
            .ok_or(ObricError::PythOffline)?;
        let price = p.scale_to_exponent(-3).ok_or(ObricError::PythError)?;
        Ok(price)
    }
}

impl AccountDeserialize for PriceFeed {
    fn try_deserialize_unchecked(data: &mut &[u8]) -> Result<Self> {
        let account: SolanaPriceAccount =
            *load_price_account(data).map_err(|_x| error!(ObricError::PythError))?;

        // Use a dummy key since the key field will be removed from the SDK
        let zero = [0u8; 32];
        let feed = account.to_price_feed(&Pubkey::from(zero));
        return Ok(PriceFeed(feed));
    }
}

pub fn parse_dove_price(
    data: &mut &[u8],
    current_time: UnixTimestamp,
    age: u8,
) -> Result<(u64, i64)> {
    let price_feed: doves_cpi::PriceFeed = doves_cpi::PriceFeed::try_deserialize(data)?;
    let mut price = price_feed.price;
    let mut expo = price_feed.expo;
    let time = price_feed.timestamp;

    if (time + age as i64) < current_time {
        return Err(ObricError::PythError.into());
    }

    if expo > -3 {
        return Err(ObricError::PythError.into());
    }

    while expo != -3 {
        expo += 1;
        price /= 10;
    }

    Ok((price, time))
}
