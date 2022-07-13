use std::collections::HashMap;

use cosmwasm_std::{to_binary, Binary, ContractResult, QuerierResult, SystemError};
use osmo_bindings::{OsmosisQuery, PoolStateResponse, SpotPriceResponse, Swap};

// NOTE: We can't use osmo_bindings::Swap (as key) for HashMap because it doesn't implement Hash
#[derive(Eq, PartialEq, Hash, Clone)]
pub struct SpotPriceKey {
    pub pool_id: u64,
    pub denom_in: String,
    pub denom_out: String,
}

impl From<&Swap> for SpotPriceKey {
    fn from(swap: &Swap) -> Self {
        Self {
            pool_id: swap.pool_id,
            denom_in: swap.denom_in.clone(),
            denom_out: swap.denom_out.clone(),
        }
    }
}

#[derive(Clone, Default)]
pub struct OsmosisQuerier {
    pub pools: HashMap<u64, PoolStateResponse>,
    pub spot_prices: HashMap<SpotPriceKey, SpotPriceResponse>,
}

impl OsmosisQuerier {
    pub fn handle_query(&self, request: OsmosisQuery) -> QuerierResult {
        let res: ContractResult<Binary> = match &request {
            OsmosisQuery::PoolState { id } => match self.pools.get(&id) {
                Some(pool_state_response) => to_binary(&pool_state_response).into(),
                None => Err(SystemError::InvalidRequest {
                    error: format!("PoolStateResponse is not found for pool id: {}", id),
                    request: Default::default(),
                })
                .into(),
            },
            OsmosisQuery::SpotPrice { swap, .. } => match self.spot_prices.get(&swap.into()) {
                Some(spot_price_response) => to_binary(&spot_price_response).into(),
                None => Err(SystemError::InvalidRequest {
                    error: format!("SpotPriceResponse is not found for swap: {:?}", swap),
                    request: Default::default(),
                })
                .into(),
            },
            _ => {
                panic!("[mock]: Unsupported Osmosis query");
            }
        };

        Ok(res).into()
    }
}
