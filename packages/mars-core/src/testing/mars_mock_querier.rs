use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockQuerier, MOCK_CONTRACT_ADDR},
    Addr, Coin, Querier, QuerierResult, QueryRequest, StdResult, SystemError, Uint128, WasmQuery,
};
use cw20::Cw20QueryMsg;
use terra_cosmwasm::TerraQueryWrapper;

use crate::{address_provider, incentives, ma_token, oracle, testing::mock_address_provider};

use super::{
    cw20_querier::{mock_token_info_response, Cw20Querier},
    incentives_querier::IncentivesQuerier,
    native_querier::NativeQuerier,
    oracle_querier::OracleQuerier,
};
use crate::math::decimal::Decimal;

pub struct MarsMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    native_querier: NativeQuerier,
    cw20_querier: Cw20Querier,
    oracle_querier: OracleQuerier,
    incentives_querier: IncentivesQuerier,
}

impl Querier for MarsMockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        // MockQuerier doesn't support Custom, so we ignore it completely here
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
                .into()
            }
        };
        self.handle_query(&request)
    }
}

impl MarsMockQuerier {
    pub fn new(base: MockQuerier<TerraQueryWrapper>) -> Self {
        MarsMockQuerier {
            base,
            native_querier: NativeQuerier::default(),
            cw20_querier: Cw20Querier::default(),
            oracle_querier: OracleQuerier::default(),
            incentives_querier: IncentivesQuerier::default(),
        }
    }

    /// Set new balances for contract address
    pub fn set_contract_balances(&mut self, contract_balances: &[Coin]) {
        let contract_addr = Addr::unchecked(MOCK_CONTRACT_ADDR);
        self.base
            .update_balance(contract_addr.to_string(), contract_balances.to_vec());
    }

    /// Set mock querier exchange rates query results for a given denom
    pub fn set_native_exchange_rates(
        &mut self,
        base_denom: String,
        exchange_rates: &[(String, Decimal)],
    ) {
        self.native_querier
            .exchange_rates
            .insert(base_denom, exchange_rates.iter().cloned().collect());
    }

    /// Set mock querier for tax data
    pub fn set_native_tax(&mut self, tax_rate: Decimal, tax_caps: &[(String, Uint128)]) {
        self.native_querier.tax_rate = tax_rate;
        self.native_querier.tax_caps = tax_caps.iter().cloned().collect();
    }

    /// Set mock querier balances results for a given cw20 token
    pub fn set_cw20_balances(&mut self, cw20_address: Addr, balances: &[(Addr, Uint128)]) {
        self.cw20_querier
            .balances
            .insert(cw20_address, balances.iter().cloned().collect());
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

    pub fn set_oracle_price(&mut self, asset_reference: Vec<u8>, price: Decimal) {
        self.oracle_querier.prices.insert(asset_reference, price);
    }

    pub fn set_incentives_address(&mut self, address: Addr) {
        self.incentives_querier.incentives_address = address;
    }

    pub fn set_unclaimed_rewards(&mut self, user_address: String, unclaimed_rewards: Uint128) {
        self.incentives_querier
            .unclaimed_rewards_at
            .insert(Addr::unchecked(user_address), unclaimed_rewards);
    }

    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match &request {
            QueryRequest::Custom(TerraQueryWrapper { route, query_data }) => {
                self.native_querier.handle_query(route, query_data)
            }

            QueryRequest::Wasm(WasmQuery::Smart { contract_addr, msg }) => {
                let contract_addr = Addr::unchecked(contract_addr);

                // Cw20 Queries
                let parse_cw20_query: StdResult<Cw20QueryMsg> = from_binary(msg);
                if let Ok(cw20_query) = parse_cw20_query {
                    return self
                        .cw20_querier
                        .handle_cw20_query(&contract_addr, cw20_query);
                }

                // MaToken Queries
                let parse_ma_token_query: StdResult<ma_token::msg::QueryMsg> = from_binary(msg);
                if let Ok(ma_token_query) = parse_ma_token_query {
                    return self
                        .cw20_querier
                        .handle_ma_token_query(&contract_addr, ma_token_query);
                }

                // Address Provider Queries
                let parse_address_provider_query: StdResult<address_provider::msg::QueryMsg> =
                    from_binary(msg);
                if let Ok(address_provider_query) = parse_address_provider_query {
                    return mock_address_provider::handle_query(
                        &contract_addr,
                        address_provider_query,
                    );
                }

                // Oracle Queries
                let parse_oracle_query: StdResult<oracle::msg::QueryMsg> = from_binary(msg);
                if let Ok(oracle_query) = parse_oracle_query {
                    return self
                        .oracle_querier
                        .handle_query(&contract_addr, oracle_query);
                }

                // Incentives Queries
                let parse_incentives_query: StdResult<incentives::msg::QueryMsg> = from_binary(msg);
                if let Ok(incentives_query) = parse_incentives_query {
                    return self
                        .incentives_querier
                        .handle_query(&contract_addr, incentives_query);
                }

                panic!("[mock]: Unsupported wasm query: {:?}", msg);
            }

            _ => self.base.handle_query(request),
        }
    }
}
