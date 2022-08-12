use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockQuerier, MOCK_CONTRACT_ADDR},
    Addr, Coin, Decimal, Empty, Querier, QuerierResult, QueryRequest, StdResult, SystemError,
    SystemResult, Uint128, WasmQuery,
};
use osmo_bindings::{OsmosisQuery, PoolStateResponse, SpotPriceResponse, Swap};

use mars_outpost::{address_provider, incentives, oracle};

use crate::incentives_querier::IncentivesQuerier;
use crate::mock_address_provider;
use crate::oracle_querier::OracleQuerier;
use crate::osmosis_querier::OsmosisQuerier;

pub struct MarsMockQuerier {
    base: MockQuerier<Empty>,
    oracle_querier: OracleQuerier,
    incentives_querier: IncentivesQuerier,
    osmosis_querier: OsmosisQuerier,
}

impl Querier for MarsMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
            }
        };

        // Custom Osmosis Queries
        let parse_osmosis_query: StdResult<QueryRequest<OsmosisQuery>> = from_slice(bin_request);
        if let Ok(QueryRequest::Custom(osmosis_query)) = parse_osmosis_query {
            return self.osmosis_querier.handle_query(osmosis_query);
        }

        self.handle_query(&request)
    }
}

impl MarsMockQuerier {
    pub fn new(base: MockQuerier<Empty>) -> Self {
        MarsMockQuerier {
            base,
            oracle_querier: OracleQuerier::default(),
            incentives_querier: IncentivesQuerier::default(),
            osmosis_querier: OsmosisQuerier::default(),
        }
    }

    /// Set new balances for contract address
    pub fn set_contract_balances(&mut self, contract_balances: &[Coin]) {
        let contract_addr = Addr::unchecked(MOCK_CONTRACT_ADDR);
        self.base.update_balance(contract_addr.to_string(), contract_balances.to_vec());
    }

    pub fn set_oracle_price(&mut self, denom: &str, price: Decimal) {
        self.oracle_querier.prices.insert(denom.to_string(), price);
    }

    pub fn set_incentives_address(&mut self, address: Addr) {
        self.incentives_querier.incentives_address = address;
    }

    pub fn set_unclaimed_rewards(&mut self, user_address: String, unclaimed_rewards: Uint128) {
        self.incentives_querier
            .unclaimed_rewards_at
            .insert(Addr::unchecked(user_address), unclaimed_rewards);
    }

    pub fn set_pool_state(&mut self, pool_id: u64, pool_state: PoolStateResponse) {
        self.osmosis_querier.pools.insert(pool_id, pool_state);
    }

    pub fn set_spot_price(&mut self, swap: Swap, spot_price: SpotPriceResponse) {
        self.osmosis_querier.spot_prices.insert(swap.into(), spot_price);
    }

    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                msg,
            }) => {
                let contract_addr = Addr::unchecked(contract_addr);

                // Address Provider Queries
                let parse_address_provider_query: StdResult<address_provider::QueryMsg> =
                    from_binary(msg);
                if let Ok(address_provider_query) = parse_address_provider_query {
                    return mock_address_provider::handle_query(
                        &contract_addr,
                        address_provider_query,
                    );
                }

                // Oracle Queries
                let parse_oracle_query: StdResult<oracle::QueryMsg> = from_binary(msg);
                if let Ok(oracle_query) = parse_oracle_query {
                    return self.oracle_querier.handle_query(&contract_addr, oracle_query);
                }

                // Incentives Queries
                let parse_incentives_query: StdResult<incentives::msg::QueryMsg> = from_binary(msg);
                if let Ok(incentives_query) = parse_incentives_query {
                    return self.incentives_querier.handle_query(&contract_addr, incentives_query);
                }

                panic!("[mock]: Unsupported wasm query: {:?}", msg);
            }

            _ => self.base.handle_query(request),
        }
    }
}
