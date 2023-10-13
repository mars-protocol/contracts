use cosmwasm_std::{Coin, Decimal};

use crate::credit_manager::ActionCoin;

pub trait Stringify {
    fn to_string(&self) -> String;
}

impl Stringify for Option<Decimal> {
    fn to_string(&self) -> String {
        self.map_or_else(|| "None".to_string(), |dec| dec.to_string())
    }
}

impl Stringify for &[Coin] {
    fn to_string(&self) -> String {
        self.iter().map(|coin| coin.to_string()).collect::<Vec<String>>().join(",")
    }
}

pub trait Denoms {
    fn to_denoms(&self) -> Vec<&str>;
}

impl Denoms for Vec<Coin> {
    fn to_denoms(&self) -> Vec<&str> {
        self.iter().map(|c| c.denom.as_str()).collect()
    }
}

impl Denoms for Vec<ActionCoin> {
    fn to_denoms(&self) -> Vec<&str> {
        self.iter().map(|c| c.denom.as_str()).collect()
    }
}

pub trait Coins {
    fn to_coins(&self) -> Vec<Coin>;
}

pub trait FallbackStr {
    fn fallback(&self, fallback: &str) -> String;
}

impl FallbackStr for String {
    fn fallback(&self, fallback: &str) -> String {
        match self {
            s if !s.is_empty() => s.clone(),
            _ => fallback.to_string(),
        }
    }
}
