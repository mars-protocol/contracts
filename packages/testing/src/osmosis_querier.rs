use std::collections::HashMap;

use cosmwasm_std::{to_binary, Binary, ContractResult, QuerierResult, SystemError};
use osmo_bindings::{
    ArithmeticTwapToNowResponse, OsmosisQuery, PoolStateResponse, SpotPriceResponse, Step, Swap,
    SwapResponse,
};

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
    pub pools: HashMap<u64, PoolStateResponse>,
    pub spot_prices: HashMap<PriceKey, SpotPriceResponse>,
    pub twap_prices: HashMap<PriceKey, ArithmeticTwapToNowResponse>,
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
            OsmosisQuery::SpotPrice {
                swap,
                ..
            } => match self.spot_prices.get(&swap.clone().into()) {
                Some(spot_price_response) => to_binary(&spot_price_response).into(),
                None => Err(SystemError::InvalidRequest {
                    error: format!("SpotPriceResponse is not found for swap: {:?}", swap),
                    request: Default::default(),
                })
                .into(),
            },
            OsmosisQuery::ArithmeticTwapToNow {
                id,
                quote_asset_denom,
                base_asset_denom,
                ..
            } => {
                let price_key = PriceKey {
                    pool_id: id,
                    denom_in: base_asset_denom,
                    denom_out: quote_asset_denom,
                };
                match self.twap_prices.get(&price_key) {
                    Some(twap_price_response) => to_binary(&twap_price_response).into(),
                    None => Err(SystemError::InvalidRequest {
                        error: format!(
                            "ArithmeticTwapToNowResponse is not found for price key: {:?}",
                            price_key
                        ),
                        request: Default::default(),
                    })
                    .into(),
                }
            }
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
        routes.join(",")
    }
}
