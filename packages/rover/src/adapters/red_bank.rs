use cosmwasm_std::{
    to_binary, Addr, Api, Coin, CosmosMsg, QuerierWrapper, QueryRequest, StdResult, Uint128,
    WasmMsg, WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use mock_red_bank::msg::{ExecuteMsg, QueryMsg, UserAssetDebtResponse};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RedBankBase<T>(T);

impl<T> RedBankBase<T> {
    pub fn new(address: T) -> RedBankBase<T> {
        RedBankBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type RedBankUnchecked = RedBankBase<String>;
pub type RedBank = RedBankBase<Addr>;

impl From<RedBank> for RedBankUnchecked {
    fn from(red_bank: RedBank) -> Self {
        Self(red_bank.0.to_string())
    }
}

impl RedBankUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<RedBank> {
        Ok(RedBankBase(api.addr_validate(&self.0)?))
    }
}

impl RedBank {
    /// Generate message for borrowing a specified amount of coin
    pub fn borrow_msg(&self, coin: &Coin) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_binary(&ExecuteMsg::Borrow {
                coin: coin.clone(),
                recipient: None,
            })?,
            funds: vec![],
        }))
    }

    pub fn query_debt(
        &self,
        querier: &QuerierWrapper,
        user_address: &Addr,
        denom: &str,
    ) -> StdResult<Uint128> {
        let response: UserAssetDebtResponse =
            querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.0.to_string(),
                msg: to_binary(&QueryMsg::UserAssetDebt {
                    user_address: user_address.to_string(),
                    denom: denom.to_string(),
                })?,
            }))?;
        Ok(response.amount)
    }
}
