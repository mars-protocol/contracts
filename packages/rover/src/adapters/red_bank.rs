use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, Api, Coin, CosmosMsg, QuerierWrapper, QueryRequest, StdResult, Uint128,
    WasmMsg, WasmQuery,
};
use mars_outpost::{red_bank, red_bank::Market};

#[cw_serde]
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
        Ok(RedBankBase(api.addr_validate(self.address())?))
    }
}

impl RedBank {
    /// Generate message for borrowing a specified amount of coin
    pub fn borrow_msg(&self, coin: &Coin) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().to_string(),
            msg: to_binary(&red_bank::ExecuteMsg::Borrow {
                denom: coin.denom.to_string(),
                amount: coin.amount,
                recipient: None,
            })?,
            funds: vec![],
        }))
    }

    /// Generate message for repaying a specified amount of coin
    pub fn repay_msg(&self, coin: &Coin) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().to_string(),
            msg: to_binary(&red_bank::ExecuteMsg::Repay {
                on_behalf_of: None,
            })?,
            funds: vec![coin.clone()],
        }))
    }

    /// Generate message for lending a specified amount of coin
    pub fn lend_msg(&self, coin: &Coin) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().to_string(),
            msg: to_binary(&red_bank::ExecuteMsg::Deposit {
                on_behalf_of: None,
            })?,
            funds: vec![coin.clone()],
        }))
    }

    pub fn query_lent(
        &self,
        querier: &QuerierWrapper,
        user_address: &Addr,
        denom: &str,
    ) -> StdResult<Uint128> {
        let response: red_bank::UserCollateralResponse =
            querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.address().to_string(),
                msg: to_binary(&red_bank::QueryMsg::UserCollateral {
                    user: user_address.to_string(),
                    denom: denom.to_string(),
                })?,
            }))?;
        Ok(response.amount)
    }

    pub fn query_debt(
        &self,
        querier: &QuerierWrapper,
        user_address: &Addr,
        denom: &str,
    ) -> StdResult<Uint128> {
        let response: red_bank::UserDebtResponse =
            querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.address().to_string(),
                msg: to_binary(&red_bank::QueryMsg::UserDebt {
                    user: user_address.to_string(),
                    denom: denom.to_string(),
                })?,
            }))?;
        Ok(response.amount)
    }

    pub fn query_market(&self, querier: &QuerierWrapper, denom: &str) -> StdResult<Market> {
        querier.query_wasm_smart(
            self.address(),
            &red_bank::QueryMsg::Market {
                denom: denom.to_string(),
            },
        )
    }
}
