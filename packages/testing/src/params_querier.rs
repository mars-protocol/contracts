use std::collections::HashMap;

use cosmwasm_std::{to_binary, Binary, ContractResult, Decimal, QuerierResult};
use mars_params::{msg::QueryMsg, types::asset::AssetParams};

#[derive(Default)]
pub struct ParamsQuerier {
    pub target_health_factor: Decimal,
    pub params: HashMap<String, AssetParams>,
}

impl ParamsQuerier {
    pub fn handle_query(&self, query: QueryMsg) -> QuerierResult {
        let ret: ContractResult<Binary> = match query {
            QueryMsg::TargetHealthFactor {} => to_binary(&self.target_health_factor).into(),
            QueryMsg::AssetParams {
                denom,
            } => match self.params.get(&denom) {
                Some(params) => to_binary(&params).into(),
                None => Err(format!("[mock]: could not find the params for {denom}")).into(),
            },
            _ => Err("[mock]: Unsupported params query".to_string()).into(),
        };
        Ok(ret).into()
    }
}
