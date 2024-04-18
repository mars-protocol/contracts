use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{to_json_binary, Addr, Coin, CosmosMsg, Env, StdResult, Uint128, WasmMsg};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    ProvideLiquidity {
        lp_token_out: String,
        recipient: Option<String>,
        minimum_receive: Uint128,
        params: Option<ZapperParams>,
    },
    WithdrawLiquidity {
        recipient: Option<String>,
        minimum_receive: Vec<Coin>,
        params: Option<ZapperParams>,
    },
    Callback(CallbackMsg),
}

#[cw_serde]
pub enum CallbackMsg {
    ReturnCoin {
        balance_before: Coin,
        recipient: Addr,
    },
}

impl CallbackMsg {
    pub fn into_cosmos_msg(self, env: &Env) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.to_string(),
            msg: to_json_binary(&ExecuteMsg::Callback(self))?,
            funds: vec![],
        }))
    }
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Uint128)]
    EstimateProvideLiquidity {
        lp_token_out: String,
        coins_in: Vec<Coin>,
        params: Option<ZapperParams>,
    },
    #[returns(Vec<Coin>)]
    EstimateWithdrawLiquidity {
        coin_in: Coin,
        params: Option<ZapperParams>,
    },
}

#[cw_serde]
pub enum ZapperParams {
    Astro(AstroParams),
}

#[cw_serde]
pub struct AstroParams {
    /// The address of the associated pair contract
    pub pair_addr: String,
    /// The address of the Astroport liquidity manager contract
    pub liquidity_manager: String,
}
