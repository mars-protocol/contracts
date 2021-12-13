use std::collections::HashMap;

use cosmwasm_std::{to_binary, Binary, ContractResult, QuerierResult, SystemError};

use astroport::{asset::PairInfo, factory::QueryMsg};

#[derive(Clone, Default)]
pub struct AstroportFactoryQuerier {
    pub pairs: HashMap<String, PairInfo>,
}

impl AstroportFactoryQuerier {
    pub fn handle_query(&self, request: &QueryMsg) -> QuerierResult {
        let ret: ContractResult<Binary> = match &request {
            QueryMsg::Pair { asset_infos } => {
                let key = format!("{}-{}", asset_infos[0], asset_infos[1]);
                match self.pairs.get(&key) {
                    Some(pair_info) => to_binary(&pair_info).into(),
                    None => Err(SystemError::InvalidRequest {
                        error: format!("PairInfo is not found for {}", key),
                        request: Default::default(),
                    })
                    .into(),
                }
            }
            _ => panic!("[mock]: Unsupported Astroport factory query"),
        };

        Ok(ret).into()
    }
}
