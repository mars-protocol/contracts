// TODO: should be removed when liquidity-helper is finalized and published to crates.io

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, Uint128};

use crate::adapters::oracle::OracleUnchecked;

#[cw_serde]
pub struct LpConfig {
    pub lp_token_denom: String,
    pub lp_pair_denoms: (String, String),
}

#[cw_serde]
pub struct InstantiateMsg {
    pub oracle: OracleUnchecked,
    pub lp_configs: Vec<LpConfig>,
}

#[cw_serde]
pub enum ExecuteMsg {
    ProvideLiquidity {
        lp_token_out: String,
        recipient: Option<String>,
        minimum_receive: Uint128,
    },
    WithdrawLiquidity {
        recipient: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Uint128)]
    EstimateProvideLiquidity {
        lp_token_out: String,
        coins_in: Vec<Coin>,
    },
    #[returns(Vec<Coin>)]
    EstimateWithdrawLiquidity {
        coin_in: Coin,
    },
}
