use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockQuerier, MOCK_CONTRACT_ADDR},
    Addr, Coin, Decimal, Empty, Querier, QuerierResult, QueryRequest, StdResult, SystemError,
    SystemResult, Uint128, WasmQuery,
};
use cw20::Cw20QueryMsg;
use osmo_bindings::{
    ArithmeticTwapToNowResponse, OsmosisQuery, PoolStateResponse, SpotPriceResponse, Swap,
};

use crate::osmosis_querier::{OsmosisQuerier, PriceKey};
use crate::{mock_address_provider, redbank_querier::RedBankQuerier};
use mars_outpost::{address_provider, incentives, ma_token, oracle, red_bank};

use super::{
    cw20_querier::{mock_token_info_response, Cw20Querier},
    incentives_querier::IncentivesQuerier,
    oracle_querier::OracleQuerier,
};

pub struct MarsMockQuerier {
    base: MockQuerier<Empty>,
    cw20_querier: Cw20Querier,
    oracle_querier: OracleQuerier,
    incentives_querier: IncentivesQuerier,
    osmosis_querier: OsmosisQuerier,
    redbank_querier: RedBankQuerier,
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
            cw20_querier: Cw20Querier::default(),
            oracle_querier: OracleQuerier::default(),
            incentives_querier: IncentivesQuerier::default(),
            osmosis_querier: OsmosisQuerier::default(),
            redbank_querier: RedBankQuerier::default(),
        }
    }

    /// Set new balances for contract address
    pub fn set_contract_balances(&mut self, contract_balances: &[Coin]) {
        let contract_addr = Addr::unchecked(MOCK_CONTRACT_ADDR);
        self.base.update_balance(contract_addr.to_string(), contract_balances.to_vec());
    }

    /// Set mock querier balances results for a given cw20 token
    pub fn set_cw20_balances(&mut self, cw20_address: Addr, balances: &[(Addr, Uint128)]) {
        self.cw20_querier.balances.insert(cw20_address, balances.iter().cloned().collect());
    }

    /// Set mock querier so that it returns a specific total supply on the token info query
    /// for a given cw20 token (note this will override existing token info with default
    /// values for the rest of the fields)
    #[allow(clippy::or_fun_call)]
    pub fn set_cw20_total_supply(&mut self, cw20_address: Addr, total_supply: Uint128) {
        let token_info = self
            .cw20_querier
            .token_info_responses
            .entry(cw20_address)
            .or_insert(mock_token_info_response());

        token_info.total_supply = total_supply;
    }

    #[allow(clippy::or_fun_call)]
    pub fn set_cw20_symbol(&mut self, cw20_address: Addr, symbol: String) {
        let token_info = self
            .cw20_querier
            .token_info_responses
            .entry(cw20_address)
            .or_insert(mock_token_info_response());

        token_info.symbol = symbol;
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

    pub fn set_redbank_market(&mut self, market: red_bank::Market) {
        self.redbank_querier.markets.insert(market.denom.clone(), market);
    }

    pub fn set_twap_price(
        &mut self,
        id: u64,
        quote_asset_denom: &str,
        base_asset_denom: &str,
        twap_price: ArithmeticTwapToNowResponse,
    ) {
        let price_key = PriceKey {
            pool_id: id,
            denom_in: base_asset_denom.to_string(),
            denom_out: quote_asset_denom.to_string(),
        };
        self.osmosis_querier.twap_prices.insert(price_key, twap_price);
    }

    pub fn handle_query(&self, request: &QueryRequest<Empty>) -> QuerierResult {
        match &request {
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                msg,
            }) => {
                let contract_addr = Addr::unchecked(contract_addr);

                // Cw20 Queries
                let parse_cw20_query: StdResult<Cw20QueryMsg> = from_binary(msg);
                if let Ok(cw20_query) = parse_cw20_query {
                    return self.cw20_querier.handle_cw20_query(&contract_addr, cw20_query);
                }

                // MaToken Queries
                let parse_ma_token_query: StdResult<ma_token::msg::QueryMsg> = from_binary(msg);
                if let Ok(ma_token_query) = parse_ma_token_query {
                    return self.cw20_querier.handle_ma_token_query(&contract_addr, ma_token_query);
                }

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

                // RedBank Queries
                if let Ok(redbank_query) = from_binary::<red_bank::QueryMsg>(msg) {
                    return self.redbank_querier.handle_query(redbank_query);
                }

                panic!("[mock]: Unsupported wasm query: {:?}", msg);
            }

            _ => self.base.handle_query(request),
        }
    }
}
