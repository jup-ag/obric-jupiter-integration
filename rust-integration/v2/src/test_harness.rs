use crate::obric_v2_amm::{id, ObricV2Amm};
use anyhow::Result;
use jupiter_amm_interface::{Amm, AmmContext, ClockRef, KeyedAccount, QuoteParams, SwapMode};
use solana_client::rpc_client::RpcClient;
use solana_sdk::clock::Clock;
use std::collections::HashMap;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AmmTestHarness {
    pub client: RpcClient,
}

impl AmmTestHarness {
    pub fn new() -> Self {
        let rpc_string = env::var("SOLANA_RPC").unwrap();
        let rpc_url = rpc_string.as_str();
        Self {
            client: RpcClient::new(rpc_url),
        }
    }

    pub fn get_all_keyed_account(&self) -> Result<Vec<KeyedAccount>> {
        let accounts = self.client.get_program_accounts(&id()).unwrap();
        let keyed_accounts = &mut vec![];
        for (key, account) in accounts {
            if account.data.len() == 666usize {
                keyed_accounts.push(KeyedAccount {
                    key,
                    account,
                    params: None,
                })
            }
        }
        Ok(keyed_accounts.clone())
    }

    pub fn update_amm(&self, amm: &mut dyn Amm) {
        let accounts_to_update = amm.get_accounts_to_update();

        let accounts_map = self
            .client
            .get_multiple_accounts(&accounts_to_update)
            .unwrap()
            .iter()
            .enumerate()
            .fold(HashMap::new(), |mut m, (index, account)| {
                if let Some(account) = account {
                    m.insert(accounts_to_update[index], account.clone());
                }
                m
            });
        amm.update(&accounts_map).unwrap();
    }
}

#[test]
fn test_quote() {
    use crate::test_harness::AmmTestHarness;
    use num::pow;

    let test_harness = AmmTestHarness::new();
    let all_keyed_account = test_harness.get_all_keyed_account().unwrap();
    let clock = Clock {
        unix_timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        ..Clock::default()
    };
    let amm_context = AmmContext {
        clock_ref: ClockRef::from(clock),
    };

    for keyed_account in all_keyed_account {
        let amm = &mut ObricV2Amm::from_keyed_account(&keyed_account, &amm_context).unwrap();
        test_harness.update_amm(amm);
        println!("Pool: {}, {}", amm.state.mint_x, amm.state.mint_y);
        let amount = pow(10, usize::from(amm.x_decimals));
        let quote = amm
            .quote(&QuoteParams {
                input_mint: amm.state.mint_x,
                amount,
                output_mint: amm.state.mint_y,
                swap_mode: SwapMode::ExactIn,
            })
            .unwrap();

        println!(
            "  Token mints: from {}, to {}",
            amm.state.mint_x, amm.state.mint_y
        );
        println!("  In amount: {}", amount);
        println!(
            "  Out amount: {:?}, Fee amount: {:?}",
            quote.out_amount, quote.fee_amount
        );

        let in_amount = pow(10, usize::from(amm.y_decimals)); // 10 SOL
        let quote = amm
            .quote(&QuoteParams {
                input_mint: amm.state.mint_y,
                amount,
                output_mint: amm.state.mint_x,
                swap_mode: SwapMode::ExactIn,
            })
            .unwrap();

        println!(
            "\n  Token mints: from {}, to {}",
            amm.state.mint_y, amm.state.mint_x
        );
        println!("  In amount: {}", in_amount);
        println!(
            "  Out amount: {:?}, Fee amount: {:?}\n",
            quote.out_amount, quote.fee_amount
        );
    }
}
