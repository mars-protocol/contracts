use std::collections::BTreeMap;
use std::fmt;

use crate::extensions::Stringify;
use cosmwasm_std::{Coin, StdError, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Pending integration into cosmwasm_std: https://github.com/CosmWasm/cosmwasm/issues/1377#issuecomment-1204232193
/// Copying from here: https://github.com/mars-protocol/cw-coins/blob/main/src/lib.rs
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Coins(pub BTreeMap<String, Uint128>);

impl From<Vec<Coin>> for Coins {
    fn from(coins: Vec<Coin>) -> Self {
        let map = coins
            .into_iter()
            .map(|coin| (coin.denom, coin.amount))
            .collect();
        Self(map)
    }
}

impl From<&[Coin]> for Coins {
    fn from(coins: &[Coin]) -> Self {
        coins.to_vec().into()
    }
}

impl Stringify for &[Coin] {
    fn to_string(&self) -> String {
        self.iter()
            .map(|coin| coin.clone().denom)
            .collect::<Vec<String>>()
            .join(", ")
    }
}

impl Coins {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn amount(&self, denom: &str) -> Option<Uint128> {
        self.0.get(denom).map(Clone::clone)
    }

    pub fn deduct(&mut self, to_deduct: &Coin) -> StdResult<&mut Self> {
        if let Some(amount) = self.amount(&to_deduct.denom) {
            let new_amount = amount.checked_sub(to_deduct.amount)?;
            if new_amount.is_zero() {
                self.0.remove(&to_deduct.denom);
            } else {
                self.0.insert(to_deduct.denom.clone(), new_amount);
            }
            Ok(self)
        } else {
            Err(StdError::generic_err(format!(
                "not found in coin list: {}",
                to_deduct.denom
            )))
        }
    }
}

impl fmt::Display for Coins {
    // TODO: For empty coins, this stringifies to am empty string, which may cause confusions.
    // Should it stringify to a more informative string, such as `[]`?
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // NOTE: The `iter` method for BTreeMap returns an Iterator where entries are already sorted
        // by key, so we don't need sort the coins manually
        let s = self
            .0
            .iter()
            .map(|(denom, amount)| format!("{}{}", amount, denom))
            .collect::<Vec<_>>()
            .join(",");
        write!(f, "{}", s)
    }
}
