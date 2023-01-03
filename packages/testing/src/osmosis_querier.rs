use std::collections::HashMap;

use cosmwasm_std::{to_binary, Binary, ContractResult, QuerierResult, SystemError};
use mars_osmosis::helpers::QueryPoolResponse;
use osmosis_std::types::osmosis::gamm::v1beta1::QueryPoolRequest;
use osmosis_std::types::osmosis::gamm::v2::{QuerySpotPriceRequest, QuerySpotPriceResponse};
use osmosis_std::types::osmosis::twap::v1beta1::{
    ArithmeticTwapToNowRequest, ArithmeticTwapToNowResponse, GeometricTwapToNowRequest,
    GeometricTwapToNowResponse,
};
use prost::{DecodeError, Message};

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct PriceKey {
    pub pool_id: u64,
    pub denom_in: String,
    pub denom_out: String,
}

#[derive(Clone, Default)]
pub struct OsmosisQuerier {
    pub pools: HashMap<u64, QueryPoolResponse>,

    pub spot_prices: HashMap<PriceKey, QuerySpotPriceResponse>,
    pub arithmetic_twap_prices: HashMap<PriceKey, ArithmeticTwapToNowResponse>,
    pub geometric_twap_prices: HashMap<PriceKey, GeometricTwapToNowResponse>,
}

impl OsmosisQuerier {
    pub fn handle_stargate_query(&self, path: &str, data: &Binary) -> Result<QuerierResult, ()> {
        if path == "/osmosis.gamm.v1beta1.Query/Pool" {
            let parse_osmosis_query: Result<QueryPoolRequest, DecodeError> =
                Message::decode(data.as_slice());
            if let Ok(osmosis_query) = parse_osmosis_query {
                return Ok(self.handle_query_pool_request(osmosis_query));
            }
        }

        if path == "/osmosis.gamm.v2.Query/SpotPrice" {
            let parse_osmosis_query: Result<QuerySpotPriceRequest, DecodeError> =
                Message::decode(data.as_slice());
            if let Ok(osmosis_query) = parse_osmosis_query {
                return Ok(self.handle_query_spot_request(osmosis_query));
            }
        }

        if path == "/osmosis.twap.v1beta1.Query/ArithmeticTwapToNow" {
            let parse_osmosis_query: Result<ArithmeticTwapToNowRequest, DecodeError> =
                Message::decode(data.as_slice());
            if let Ok(osmosis_query) = parse_osmosis_query {
                return Ok(self.handle_query_arithmetic_twap_request(osmosis_query));
            }
        }

        if path == "/osmosis.twap.v1beta1.Query/GeometricTwapToNow" {
            let parse_osmosis_query: Result<GeometricTwapToNowRequest, DecodeError> =
                Message::decode(data.as_slice());
            if let Ok(osmosis_query) = parse_osmosis_query {
                return Ok(self.handle_query_geometric_twap_request(osmosis_query));
            }
        }

        Err(())
    }

    fn handle_query_pool_request(&self, request: QueryPoolRequest) -> QuerierResult {
        let pool_id = request.pool_id;
        let res: ContractResult<Binary> = match self.pools.get(&pool_id) {
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

    fn handle_query_arithmetic_twap_request(
        &self,
        request: ArithmeticTwapToNowRequest,
    ) -> QuerierResult {
        let price_key = PriceKey {
            pool_id: request.pool_id,
            denom_in: request.base_asset,
            denom_out: request.quote_asset,
        };
        let res: ContractResult<Binary> = match self.arithmetic_twap_prices.get(&price_key) {
            Some(query_response) => to_binary(&query_response).into(),
            None => Err(SystemError::InvalidRequest {
                error: format!(
                    "ArithmeticTwapToNowResponse is not found for price key: {:?}",
                    price_key
                ),
                request: Default::default(),
            })
            .into(),
        };
        Ok(res).into()
    }

    fn handle_query_geometric_twap_request(
        &self,
        request: GeometricTwapToNowRequest,
    ) -> QuerierResult {
        let price_key = PriceKey {
            pool_id: request.pool_id,
            denom_in: request.base_asset,
            denom_out: request.quote_asset,
        };
        let res: ContractResult<Binary> = match self.geometric_twap_prices.get(&price_key) {
            Some(query_response) => to_binary(&query_response).into(),
            None => Err(SystemError::InvalidRequest {
                error: format!(
                    "GeometricTwapToNowResponse is not found for price key: {:?}",
                    price_key
                ),
                request: Default::default(),
            })
            .into(),
        };
        Ok(res).into()
    }
}
