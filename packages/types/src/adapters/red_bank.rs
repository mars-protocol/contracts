use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Api, Coin, CosmosMsg, QuerierWrapper, QueryRequest, StdResult, Uint128,
    WasmMsg, WasmQuery,
};

use crate::red_bank;

#[cw_serde]
pub struct RedBankUnchecked(String);

impl RedBankUnchecked {
    pub fn new(address: String) -> Self {
        Self(address)
    }

    pub fn address(&self) -> &str {
        &self.0
    }

    pub fn check(&self, api: &dyn Api, credit_manager: Addr) -> StdResult<RedBank> {
        let addr = api.addr_validate(self.address())?;
        Ok(RedBank::new(addr, credit_manager))
    }
}

impl From<RedBank> for RedBankUnchecked {
    fn from(red_bank: RedBank) -> Self {
        Self(red_bank.addr.to_string())
    }
}

#[cw_serde]
pub struct RedBank {
    pub addr: Addr,
    credit_manager: Addr,
}

impl RedBank {
    pub fn new(addr: Addr, credit_manager: Addr) -> Self {
        Self {
            addr,
            credit_manager,
        }
    }
}

impl RedBank {
    /// Generate message for borrowing a specified amount of coin
    pub fn borrow_msg(&self, coin: &Coin) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.addr.to_string(),
            msg: to_json_binary(&red_bank::ExecuteMsg::Borrow {
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
            contract_addr: self.addr.to_string(),
            msg: to_json_binary(&red_bank::ExecuteMsg::Repay {
                on_behalf_of: None,
            })?,
            funds: vec![coin.clone()],
        }))
    }

    /// Generate message for lending a specified amount of coin
    pub fn lend_msg(&self, coin: &Coin, account_id: &str) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.addr.to_string(),
            msg: to_json_binary(&red_bank::ExecuteMsg::Deposit {
                account_id: Some(account_id.to_string()),
                on_behalf_of: None,
            })?,
            funds: vec![coin.clone()],
        }))
    }

    /// Generate message for reclaiming a specified amount of lent coin
    pub fn reclaim_msg(
        &self,
        coin: &Coin,
        account_id: &str,
        liquidation_related: bool,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.addr.to_string(),
            msg: to_json_binary(&red_bank::ExecuteMsg::Withdraw {
                denom: coin.denom.clone(),
                amount: Some(coin.amount),
                recipient: None,
                account_id: Some(account_id.to_string()),
                liquidation_related: Some(liquidation_related),
            })?,
            funds: vec![],
        }))
    }

    pub fn query_lent(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
        denom: &str,
    ) -> StdResult<Uint128> {
        let response: red_bank::UserCollateralResponse =
            querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.addr.to_string(),
                msg: to_json_binary(&red_bank::QueryMsg::UserCollateral {
                    user: self.credit_manager.to_string(),
                    account_id: Some(account_id.to_string()),
                    denom: denom.to_string(),
                })?,
            }))?;
        Ok(response.amount)
    }

    pub fn query_all_lent(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
    ) -> StdResult<Vec<Coin>> {
        let mut start_after = Option::<String>::None;
        let mut has_more = true;
        let mut all_lent_coins = Vec::new();
        while has_more {
            let response = self.query_lents(querier, account_id, start_after, None)?;
            for item in response.data {
                all_lent_coins.push(Coin {
                    denom: item.denom,
                    amount: item.amount,
                });
            }
            start_after = all_lent_coins.last().map(|item| item.denom.clone());
            has_more = response.metadata.has_more;
        }
        Ok(all_lent_coins)
    }

    fn query_lents(
        &self,
        querier: &QuerierWrapper,
        account_id: &str,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<red_bank::PaginatedUserCollateralResponse> {
        querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: self.addr.to_string(),
            msg: to_json_binary(&red_bank::QueryMsg::UserCollateralsV2 {
                user: self.credit_manager.to_string(),
                account_id: Some(account_id.to_string()),
                start_after,
                limit,
            })?,
        }))
    }

    pub fn query_debt(&self, querier: &QuerierWrapper, denom: &str) -> StdResult<Uint128> {
        let response: red_bank::UserDebtResponse =
            querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.addr.to_string(),
                msg: to_json_binary(&red_bank::QueryMsg::UserDebt {
                    user: self.credit_manager.to_string(),
                    denom: denom.to_string(),
                })?,
            }))?;
        Ok(response.amount)
    }
}
