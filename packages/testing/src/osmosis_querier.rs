use std::collections::HashMap;

use cosmwasm_std::{to_binary, Binary, ContractResult, QuerierResult, SystemError};
use osmo_bindings::{OsmosisQuery, PoolStateResponse, Step, Swap, SwapResponse};
use osmosis_std::types::osmosis::gamm::twap::v1beta1::{
    GetArithmeticTwapRequest, GetArithmeticTwapResponse,
};
use osmosis_std::types::osmosis::gamm::v1beta1::{
    QueryPoolRequest, QueryPoolResponse, QuerySpotPriceRequest, QuerySpotPriceResponse,
};
use prost::{DecodeError, Message};

// NOTE: We can't use osmo_bindings::Swap (as key) for HashMap because it doesn't implement Hash
#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct PriceKey {
    pub pool_id: u64,
    pub denom_in: String,
    pub denom_out: String,
}

impl From<Swap> for PriceKey {
    fn from(swap: Swap) -> Self {
        Self {
            pool_id: swap.pool_id,
            denom_in: swap.denom_in,
            denom_out: swap.denom_out,
        }
    }
}

#[derive(Clone, Default)]
pub struct OsmosisQuerier {
    // NOTE: two types of deps used, will be simplified when all feature including TWAP is in osmosis-rust
    // osmosis-bindings
    pub pools: HashMap<u64, PoolStateResponse>,
    // osmosis-rust
    pub pool_responses: HashMap<u64, QueryPoolResponse>,

    pub spot_prices: HashMap<PriceKey, QuerySpotPriceResponse>,
    pub twap_prices: HashMap<PriceKey, GetArithmeticTwapResponse>,
    /// key comes from `prepare_estimate_swap_key` function
    pub estimate_swaps: HashMap<String, SwapResponse>,
}

impl OsmosisQuerier {
    pub fn handle_query(&self, request: OsmosisQuery) -> QuerierResult {
        let res: ContractResult<Binary> = match request {
            OsmosisQuery::PoolState {
                id,
            } => match self.pools.get(&id) {
                Some(pool_state_response) => to_binary(&pool_state_response).into(),
                None => Err(SystemError::InvalidRequest {
                    error: format!("PoolStateResponse is not found for pool id: {}", id),
                    request: Default::default(),
                })
                .into(),
            },
            OsmosisQuery::EstimateSwap {
                first,
                route,
                ..
            } => {
                let routes_key = Self::prepare_estimate_swap_key(&first, &route);
                match self.estimate_swaps.get(&routes_key) {
                    Some(swap_response) => to_binary(&swap_response).into(),
                    None => Err(SystemError::InvalidRequest {
                        error: format!("SwapResponse is not found for routes: {:?}", routes_key),
                        request: Default::default(),
                    })
                    .into(),
                }
            }
            _ => {
                panic!("[mock]: Unsupported Osmosis query");
            }
        };

        Ok(res).into()
    }

    pub fn prepare_estimate_swap_key(first: &Swap, route: &[Step]) -> String {
        let routes: Vec<_> = vec![Step {
            pool_id: first.pool_id,
            denom_out: first.denom_out.clone(),
        }]
        .iter()
        .chain(route)
        .map(|step| format!("{}.{}", step.pool_id, step.denom_out))
        .collect();
        let routes = routes.join(",");
        format!("{},{}", first.denom_in, routes)
    }

    pub fn handle_stargate_query(&self, path: &str, data: &Binary) -> Result<QuerierResult, ()> {
        if path == "/osmosis.gamm.v1beta1.Query/Pool" {
            let parse_osmosis_query: Result<QueryPoolRequest, DecodeError> =
                Message::decode(data.as_slice());
            if let Ok(osmosis_query) = parse_osmosis_query {
                return Ok(self.handle_query_pool_request(osmosis_query));
            }
        }

        if path == "/osmosis.gamm.v1beta1.Query/SpotPrice" {
            let parse_osmosis_query: Result<QuerySpotPriceRequest, DecodeError> =
                Message::decode(data.as_slice());
            if let Ok(osmosis_query) = parse_osmosis_query {
                return Ok(self.handle_query_spot_request(osmosis_query));
            }
        }

        if path == "/osmosis.gamm.twap.v1beta1.Query/GetArithmeticTwap" {
            let parse_osmosis_query: Result<GetArithmeticTwapRequest, DecodeError> =
                Message::decode(data.as_slice());
            if let Ok(osmosis_query) = parse_osmosis_query {
                return Ok(self.handle_query_twap_request(osmosis_query));
            }
        }

        Err(())
    }

    fn handle_query_pool_request(&self, request: QueryPoolRequest) -> QuerierResult {
        let pool_id = request.pool_id;
        let res: ContractResult<Binary> = match self.pool_responses.get(&pool_id) {
            Some(query_response) => to_binary(&query_response).into(),
            None => Err(SystemError::InvalidRequest {
                error: format!("QueryPoolResponse is not found for pool id: {}", pool_id),
                request: Default::default(),
            })
            .into(),
        };
        Ok(res).into()
    }

    fn handle_query_spot_request(&self, request: QuerySpotPriceRequest) -> QuerierResult {
        let price_key = PriceKey {
            pool_id: request.pool_id,
            denom_in: request.base_asset_denom,
            denom_out: request.quote_asset_denom,
        };
        let res: ContractResult<Binary> = match self.spot_prices.get(&price_key) {
            Some(query_response) => to_binary(&query_response).into(),
            None => Err(SystemError::InvalidRequest {
                error: format!(
                    "QuerySpotPriceResponse is not found for price key: {:?}",
                    price_key
                ),
                request: Default::default(),
            })
            .into(),
        };
        Ok(res).into()
    }

    fn handle_query_twap_request(&self, request: GetArithmeticTwapRequest) -> QuerierResult {
        let price_key = PriceKey {
            pool_id: request.pool_id,
            denom_in: request.base_asset,
            denom_out: request.quote_asset,
        };
        let res: ContractResult<Binary> = match self.twap_prices.get(&price_key) {
            Some(query_response) => to_binary(&query_response).into(),
            None => Err(SystemError::InvalidRequest {
                error: format!(
                    "GetArithmeticTwapResponse is not found for price key: {:?}",
                    price_key
                ),
                request: Default::default(),
            })
            .into(),
        };
        Ok(res).into()
    }
}
