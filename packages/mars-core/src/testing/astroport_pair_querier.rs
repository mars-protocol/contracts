use std::collections::HashMap;

use cosmwasm_std::{to_binary, Addr, Binary, ContractResult, QuerierResult, SystemError};

use astroport::pair::{CumulativePricesResponse, PoolResponse, QueryMsg, SimulationResponse};

#[derive(Clone, Default)]
pub struct AstroportPairQuerier {
    pub pairs: HashMap<String, PoolResponse>,
    pub simulations: HashMap<String, SimulationResponse>,
    pub cumulative_prices: HashMap<String, CumulativePricesResponse>,
}

impl AstroportPairQuerier {
    pub fn handle_query(&self, contract_addr: &Addr, request: &QueryMsg) -> QuerierResult {
        let key = contract_addr.to_string();
        let ret: ContractResult<Binary> = match &request {
            QueryMsg::Pool {} => match self.pairs.get(&key) {
                Some(pool_response) => to_binary(&pool_response).into(),
                None => Err(SystemError::InvalidRequest {
                    error: format!("PoolResponse is not found for {}", key),
                    request: Default::default(),
                })
                .into(),
            },
            QueryMsg::CumulativePrices {} => match self.cumulative_prices.get(&key) {
                Some(cumulative_prices_response) => to_binary(&cumulative_prices_response).into(),
                None => Err(SystemError::InvalidRequest {
                    error: format!("CumulativePricesResponse is not found for {}", key),
                    request: Default::default(),
                })
                .into(),
            },
            QueryMsg::Simulation { .. } => match self.simulations.get(&key) {
                Some(simulation_response) => to_binary(&simulation_response).into(),
                None => Err(SystemError::InvalidRequest {
                    error: format!("SimulationResponse is not found for {}", key),
                    request: Default::default(),
                })
                .into(),
            },
            _ => {
                panic!("[mock]: Unsupported Astroport pair query");
            }
        };

        Ok(ret).into()
    }
}
