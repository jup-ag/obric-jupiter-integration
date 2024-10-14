use anchor_lang::prelude::*;
use pyth_sdk::{Price, UnixTimestamp};
use pyth_sdk_solana::state::{load_price_account, SolanaPriceAccount};

use crate::errors::ObricError;

#[derive(Clone, Debug)]
pub struct PriceFeed(pyth_sdk::PriceFeed);

impl PriceFeed {
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

pub fn parse_dove_price(data: &[u8], current_time: UnixTimestamp, age: u8) -> Option<(u64, i64)> {

    if data.len() != 283 {
        return None;
    }

    let mut price = unsafe {
        data.as_ptr().add(8 + 32 + 33).cast::<u64>().read_unaligned()
    };

    let mut expo = unsafe {
        data.as_ptr().add(8 + 32 + 33 + 8).cast::<i8>().read_unaligned()
    };

    let time = unsafe {
        data.as_ptr().add(8 + 32 + 33 + 8 + 1).cast::<i64>().read_unaligned()
    };

    if (time + age as i64) < current_time {
        return None;
    }

    if expo > -3 {
        return None;
    }

    while expo != -3 {
        expo += 1;
        price /= 10;
    }

    Some((price, time))
}
