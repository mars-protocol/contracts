use cosmwasm_std::{
    to_binary, Addr, Api, CosmosMsg, QuerierWrapper, QueryRequest, StdResult, Uint128, WasmMsg,
    WasmQuery,
};
use cw_asset::{Asset, AssetInfo};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use mock_red_bank::msg::{ExecuteMsg, QueryMsg, UserAssetDebtResponse};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RedBankBase<T> {
    pub contract_addr: T,
}

pub type RedBankUnchecked = RedBankBase<String>;
pub type RedBank = RedBankBase<Addr>;

impl From<RedBank> for RedBankUnchecked {
    fn from(red_bank: RedBank) -> Self {
        RedBankUnchecked {
            contract_addr: red_bank.contract_addr.to_string(),
        }
    }
}

impl RedBankUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<RedBank> {
        Ok(RedBank {
            contract_addr: api.addr_validate(&self.contract_addr)?,
        })
    }
}

impl RedBank {
    /// Generate message for borrowing a specified amount of asset
    pub fn borrow_msg(&self, asset: &Asset) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.contract_addr.to_string(),
            msg: to_binary(&ExecuteMsg::Borrow {
                asset: asset.clone(),
                recipient: None,
            })?,
            funds: vec![],
        }))
    }

    pub fn query_user_debt(
        &self,
        querier: &QuerierWrapper,
        user_address: &Addr,
        asset_info: &AssetInfo,
    ) -> StdResult<Uint128> {
        let response: UserAssetDebtResponse =
            querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: self.contract_addr.to_string(),
                msg: to_binary(&QueryMsg::UserAssetDebt {
                    user_address: user_address.to_string(),
                    asset: asset_info.clone().into(),
                })?,
            }))?;
        Ok(response.amount)
    }
}
