use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, Api, Coin, CosmosMsg, QuerierWrapper, StdResult, Uint128, WasmMsg,
};

use crate::msg::zapper::{ExecuteMsg, QueryMsg};

#[cw_serde]
pub struct ZapperBase<T>(T);

impl<T> ZapperBase<T> {
    pub fn new(address: T) -> ZapperBase<T> {
        ZapperBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type ZapperUnchecked = ZapperBase<String>;
pub type Zapper = ZapperBase<Addr>;

impl From<Zapper> for ZapperUnchecked {
    fn from(zapper: Zapper) -> Self {
        Self(zapper.address().to_string())
    }
}

impl ZapperUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<Zapper> {
        Ok(ZapperBase::new(api.addr_validate(self.address())?))
    }
}

impl Zapper {
    pub fn estimate_provide_liquidity(
        &self,
        querier: &QuerierWrapper,
        lp_token_out: &str,
        coins_in: &[Coin],
    ) -> StdResult<Uint128> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::EstimateProvideLiquidity {
                lp_token_out: lp_token_out.to_string(),
                coins_in: coins_in.to_vec(),
            },
        )
    }

    pub fn estimate_withdraw_liquidity(
        &self,
        querier: &QuerierWrapper,
        lp_token: &Coin,
    ) -> StdResult<Vec<Coin>> {
        querier.query_wasm_smart(
            self.address().to_string(),
            &QueryMsg::EstimateWithdrawLiquidity {
                coin_in: lp_token.clone(),
            },
        )
    }

    pub fn provide_liquidity_msg(
        &self,
        coins_in: &[Coin],
        lp_token_out: &str,
        minimum_receive: Uint128,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().to_string(),
            msg: to_binary(&ExecuteMsg::ProvideLiquidity {
                lp_token_out: lp_token_out.to_string(),
                minimum_receive,
                recipient: None,
            })?,
            funds: coins_in.to_vec(),
        }))
    }

    pub fn withdraw_liquidity_msg(&self, lp_token: &Coin) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().to_string(),
            msg: to_binary(&ExecuteMsg::WithdrawLiquidity { recipient: None })?,
            funds: vec![lp_token.clone()],
        }))
    }
}
