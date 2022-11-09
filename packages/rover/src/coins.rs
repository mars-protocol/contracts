use std::any::type_name;
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::str::FromStr;

use crate::traits::{Denoms, Stringify};
use cosmwasm_std::{Coin, StdError, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{de, Serialize};

/// A collection of coins, similar to Cosmos SDK's `sdk.Coins` struct.
///
/// Differently from `sdk.Coins`, which is a vector of `sdk.Coin`, here we implement Coins as a BTreeMap
/// that maps from coin denoms to amounts. This has a number of advantages:
///
/// * coins are naturally sorted alphabetically by denom
/// * duplicate denoms are automatically removed
/// * cheaper for searching/inserting/deleting: O(log(n)) compared to O(n)
/// * compared to `Vec<Coin>`, the map data structure stringifies to a compact JSON representation,
///   therefore is cheaper when writing to contract storage
///
/// ## On the string representation of coins
///
/// Two approaches are implemented for stringifing Coins: the JSON representation, and the plain text
/// representation.
///
/// **The JSON representation** comes in the format below. This is used for contract storage or message
/// passing between contracts:
///
/// ```json
/// {"uatom":"12345","umars":"42069","uosmo":"88888"}
/// ```
///
/// Use the `serde_json` library to convert Coins to/from JSON strings:
///
/// ```rust
/// use cw_coins::Coins;
///
/// let coins: Coins = serde_json::from_str(r#"{"uatom":"12345","uosmo":"42069"}"#).unwrap();
/// let json = serde_json::to_string(&coins).unwrap();
/// ```
///
/// The plain text representation is the same format as the `sdk.Coins.String` method uses. It is used
/// in event logging:
///
/// ```plain
/// 12345uatom,42069umars,88888uosmo
/// ```
///
/// Use `{from,to}_string` methods to convert Coin to/from plain strings:
///
/// ```rust
/// use std::str::FromStr;
/// use cw_coins::Coins;
///
/// let coins = Coins::from_str("12345uatom,42069umars,88888uosmo").unwrap();
/// let plain = coins.to_string();
/// ```
#[derive(Serialize, Clone, Default, Debug, PartialEq, Eq, JsonSchema)]
pub struct Coins(BTreeMap<String, Uint128>);

// We implement a custom serde::de::Deserialize trait to handle the case where the JSON string contains
// duplicate keys, i.e. duplicate coin denoms.
//
// If we derive the trait, by default, it will not throw an error in such a case. Instead, it takes
// the amount that is seen the last. E.g. the following JSON string
//
// ```json
// {
//    "uatom": "12345",
//    "uatom", "23456",
//    "uatom": "67890"
// }
// ```
//
// will be deserialized into a Coins object with only one element, with denom `uatom` and amount 67890.
// The amount 67890 is seen the last and overwrites the two amounts seen earlier.
//
// This is NOT a desirable property. We want an error to be thown if the JSON string contain dups.
impl<'de> de::Deserialize<'de> for Coins {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Coins;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a map with non-duplicating string keys and stringified 128-bit unsigned integer values")
            }

            #[inline]
            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                let mut seen_denoms = HashSet::<String>::new();
                let mut coins = BTreeMap::<String, Uint128>::new();

                while let Some((denom, amount_str)) = access.next_entry::<String, String>()? {
                    if seen_denoms.contains(&denom) {
                        return Err(de::Error::custom(format!(
                            "failed to parse into Coins! duplicate denom: {}",
                            denom
                        )));
                    }

                    let amount = Uint128::from_str(&amount_str).map_err(|_| {
                        de::Error::custom(format!(
                            "failed to parse into Coins! invalid amount: {}",
                            amount_str
                        ))
                    })?;

                    if amount.is_zero() {
                        return Err(de::Error::custom(format!(
                            "amount for denom {} is zero",
                            denom
                        )));
                    }

                    seen_denoms.insert(denom.clone());
                    coins.insert(denom, amount);
                }

                Ok(Coins(coins))
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

impl TryFrom<Vec<Coin>> for Coins {
    type Error = StdError;

    fn try_from(vec: Vec<Coin>) -> StdResult<Self> {
        let vec_len = vec.len();
        let map = vec
            .into_iter()
            .filter(|coin| !coin.amount.is_zero())
            .map(|coin| (coin.denom, coin.amount))
            .collect::<BTreeMap<_, _>>();

        // the map having a different length from the vec means the vec must either 1) contain
        // duplicate denoms, or 2) contain zero amounts
        if map.len() != vec_len {
            return Err(StdError::parse_err(
                type_name::<Self>(),
                "duplicate denoms or zero amount",
            ));
        }

        Ok(Self(map))
    }
}

impl TryFrom<&[Coin]> for Coins {
    type Error = StdError;

    fn try_from(slice: &[Coin]) -> StdResult<Self> {
        slice.to_vec().try_into()
    }
}

impl FromStr for Coins {
    type Err = StdError;

    fn from_str(s: &str) -> StdResult<Self> {
        // `cosmwasm_std::Coin` does not implement `FromStr`, so we have do it ourselves
        //
        // Parsing the string with regex doesn't work, because the resulting wasm binary would be
        // too big from including the `regex` library.
        //
        // If the binary size is not a concern, here's an example:
        // https://github.com/PFC-Validator/terra-rust/blob/v1.1.8/terra-rust-api/src/client/core_types.rs#L34-L55
        //
        // We opt for the following solution: enumerate characters in the string, and break before
        // the first non-number character. Split the string at that index.
        //
        // This assumes the denom never starts with a number, which is the case:
        // https://github.com/cosmos/cosmos-sdk/blob/v0.46.0/types/coin.go#L854-L856
        let parse_coin_str = |s: &str| -> StdResult<Coin> {
            for (i, c) in s.chars().enumerate() {
                if c.is_alphabetic() {
                    let amount = Uint128::from_str(&s[..i])?;
                    let denom = String::from(&s[i..]);
                    return Ok(Coin { amount, denom });
                }
            }

            Err(StdError::parse_err(
                type_name::<Coin>(),
                format!("invalid coin string: {s}"),
            ))
        };

        s.split(',')
            .into_iter()
            .map(parse_coin_str)
            .collect::<StdResult<Vec<_>>>()?
            .try_into()
    }
}

impl fmt::Display for Coins {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // NOTE: The `iter` method for BTreeMap returns an Iterator where entries are already sorted
        // by key, so we don't need to sort the coins manually
        let s = self
            .0
            .iter()
            .map(|(denom, amount)| format!("{amount}{denom}"))
            .collect::<Vec<_>>()
            .join(",");
        write!(f, "{s}")
    }
}

impl Coins {
    /// Cast to Vec<Coin>, while NOT consuming the original object
    pub fn to_vec(&self) -> Vec<Coin> {
        self.0
            .iter()
            .map(|(denom, amount)| Coin {
                denom: denom.clone(),
                amount: *amount,
            })
            .collect()
    }

    /// Cast to Vec<Coin>, consuming the original object
    pub fn into_vec(self) -> Vec<Coin> {
        self.0
            .into_iter()
            .map(|(denom, amount)| Coin { denom, amount })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn denoms(&self) -> Vec<String> {
        self.0.keys().cloned().collect()
    }

    pub fn amount(&self, denom: &str) -> Option<Uint128> {
        self.0.get(denom).map(Clone::clone)
    }

    /// NOTE: the syntax can be simpler if Uint128 has an inplace add method...
    pub fn add(&mut self, coin: &Coin) -> StdResult<()> {
        let amount = self
            .0
            .entry(coin.denom.clone())
            .or_insert_with(Uint128::zero);
        *amount = amount.checked_add(coin.amount)?;
        Ok(())
    }

    pub fn deduct(&mut self, to_deduct: &Coin) -> StdResult<()> {
        if let Some(amount) = self.amount(&to_deduct.denom) {
            let new_amount = amount.checked_sub(to_deduct.amount)?;
            if new_amount.is_zero() {
                self.0.remove(&to_deduct.denom);
            } else {
                self.0.insert(to_deduct.denom.clone(), new_amount);
            }
            Ok(())
        } else {
            Err(StdError::generic_err(format!(
                "not found in coin list: {}",
                to_deduct.denom
            )))
        }
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

impl Denoms for Vec<Coin> {
    fn to_denoms(&self) -> Vec<&str> {
        self.iter().map(|c| c.denom.as_str()).collect()
    }
}
