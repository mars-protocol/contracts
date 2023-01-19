use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use cw_utils::Duration;
use mars_rover::msg::execute::{ActionAmount, ActionCoin};

#[cw_serde]
pub struct AccountToFund {
    pub addr: Addr,
    pub funds: Vec<Coin>,
}

#[cw_serde]
pub struct CoinInfo {
    pub denom: String,
    pub price: Decimal,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
    pub liquidation_bonus: Decimal,
}

#[cw_serde]
pub struct LpCoinInfo {
    pub denom: String,
    pub price: Decimal,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
    pub underlying_pair: (String, String),
}

#[cw_serde]
pub struct VaultTestInfo {
    pub vault_token_denom: String,
    pub base_token_denom: String,
    pub lockup: Option<Duration>,
    pub deposit_cap: Coin,
    pub max_ltv: Decimal,
    pub liquidation_threshold: Decimal,
    pub whitelisted: bool,
}

impl CoinInfo {
    pub fn to_coin(&self, amount: u128) -> Coin {
        coin(amount, self.denom.clone())
    }

    pub fn to_action_coin(&self, amount: u128) -> ActionCoin {
        ActionCoin {
            denom: self.denom.clone(),
            amount: ActionAmount::Exact(Uint128::new(amount)),
        }
    }

    pub fn to_action_coin_full_balance(&self) -> ActionCoin {
        ActionCoin {
            denom: self.denom.clone(),
            amount: ActionAmount::AccountBalance,
        }
    }
}
