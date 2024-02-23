use std::collections::HashMap;

use cosmwasm_std::{to_json_binary, Binary, Coin, ContractResult, Decimal, QuerierResult, Uint128};
use mars_types::params::{AssetParams, QueryMsg};

#[derive(Default)]
pub struct ParamsQuerier {
    pub target_health_factor: Decimal,
    pub params: HashMap<String, AssetParams>,
    pub total_deposits: HashMap<String, Uint128>,
}

impl ParamsQuerier {
    pub fn handle_query(&self, query: QueryMsg) -> QuerierResult {
        let ret: ContractResult<Binary> = match query {
            QueryMsg::TargetHealthFactor {} => to_json_binary(&self.target_health_factor).into(),
            QueryMsg::AssetParams {
                denom,
            } => match self.params.get(&denom) {
                Some(params) => to_json_binary(&params).into(),
                None => Err(format!("[mock]: could not find the params for {denom}")).into(),
            },
            QueryMsg::TotalDeposit {
                denom,
            } => match self.total_deposits.get(&denom) {
                Some(amount) => to_json_binary(&Coin {
                    denom,
                    amount: *amount,
                })
                .into(),
                None => Err(format!("[mock]: could not find total deposit for {denom}")).into(),
            },
            _ => Err("[mock]: Unsupported params query".to_string()).into(),
        };
        Ok(ret).into()
    }
}
