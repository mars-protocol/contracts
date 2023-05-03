use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, QuerierWrapper, StdResult, Uint128, WasmMsg};

use crate::msg::{ExecuteMsg, QueryMsg};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Zapper(pub Addr);

impl Zapper {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }

    pub fn provide_liquidity(
        &self,
        lp_token_out: String,
        recipient: Option<String>,
        minimum_receive: Uint128,
    ) -> StdResult<CosmosMsg> {
        self.call(ExecuteMsg::ProvideLiquidity {
            lp_token_out,
            recipient,
            minimum_receive,
        })
    }

    pub fn withdraw_liquidity(&self, recipient: Option<String>) -> StdResult<CosmosMsg> {
        self.call(ExecuteMsg::WithdrawLiquidity { recipient })
    }

    pub fn estimate_provide_liquidity(
        &self,
        querier: &QuerierWrapper,
        lp_token_out: String,
        coins_in: Vec<Coin>,
    ) -> StdResult<Uint128> {
        querier.query_wasm_smart(
            self.0.to_string(),
            &QueryMsg::EstimateProvideLiquidity {
                lp_token_out,
                coins_in,
            },
        )
    }

    pub fn estimate_withdraw_liquidity(
        &self,
        querier: &QuerierWrapper,
        coin_in: Coin,
    ) -> StdResult<Vec<Coin>> {
        querier.query_wasm_smart(
            self.0.to_string(),
            &QueryMsg::EstimateWithdrawLiquidity { coin_in },
        )
    }
}
