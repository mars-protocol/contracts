use std::str::FromStr;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128};
use cw_utils::Duration;
use mars_params::types::{AssetParams, HighLeverageStrategyParams, RedBankSettings, RoverSettings};
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
    pub whitelisted: bool,
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

impl From<CoinInfo> for AssetParams {
    fn from(c: CoinInfo) -> Self {
        Self {
            denom: c.denom,
            rover: RoverSettings {
                whitelisted: c.whitelisted,
                hls: HighLeverageStrategyParams {
                    max_loan_to_value: Decimal::from_str("0.86").unwrap(),
                    liquidation_threshold: Decimal::from_str("0.89").unwrap(),
                },
            },
            red_bank: RedBankSettings {
                deposit_enabled: true,
                borrow_enabled: true,
                deposit_cap: Uint128::MAX,
            },
            max_loan_to_value: c.max_ltv,
            liquidation_threshold: c.liquidation_threshold,
            liquidation_bonus: c.liquidation_bonus,
        }
    }
}
