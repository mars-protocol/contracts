use std::fmt;

use cosmwasm_std::{Coin, StdError, StdResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Partial integration of cw-asset's `AssetList` but just for native `Coin`
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CoinList(Vec<Coin>);

impl From<&Vec<Coin>> for CoinList {
    fn from(coins: &Vec<Coin>) -> Self {
        Self(coins.clone())
    }
}

impl CoinList {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn find_denom(&self, denom: &str) -> Option<&Coin> {
        self.0.iter().find(|item| item.denom == denom)
    }

    pub fn deduct(&mut self, to_deduct: &Coin) -> StdResult<&mut Self> {
        match self.0.iter_mut().find(|coin| coin.denom == to_deduct.denom) {
            Some(coin) => {
                coin.amount = coin.amount.checked_sub(to_deduct.amount)?;
            }
            None => {
                return Err(StdError::generic_err(format!(
                    "not found in asset list: {}",
                    to_deduct.denom
                )));
            }
        }
        Ok(self.purge())
    }

    pub fn purge(&mut self) -> &mut Self {
        self.0.retain(|coin| !coin.amount.is_zero());
        self
    }
}

impl fmt::Display for CoinList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|coin| coin.to_string())
                .collect::<Vec<String>>()
                .join(",")
        )
    }
}
