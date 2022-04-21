use cosmwasm_std::{
    from_binary, from_slice,
    testing::{MockQuerier, MOCK_CONTRACT_ADDR},
    Addr, Coin, Fraction, Querier, QuerierResult, QueryRequest, StdResult, SystemError, Uint128,
    WasmQuery,
};
use cw20::Cw20QueryMsg;
use terra_cosmwasm::TerraQueryWrapper;

use crate::{
    address_provider, incentives, ma_token, oracle, staking, testing::mock_address_provider,
    vesting, xmars_token,
};
use astroport::{
    asset::{Asset, PairInfo},
    pair::{CumulativePricesResponse, PoolResponse, SimulationResponse},
};

use super::{
    astroport_factory_querier::AstroportFactoryQuerier,
    astroport_pair_querier::AstroportPairQuerier,
    cw20_querier::{mock_token_info_response, Cw20Querier},
    incentives_querier::IncentivesQuerier,
    native_querier::NativeQuerier,
    oracle_querier::OracleQuerier,
    staking_querier::StakingQuerier,
    vesting_querier::VestingQuerier,
    xmars_querier::XMarsQuerier,
};
use crate::math::decimal::Decimal;
use crate::testing::basset_querier::BAssetQuerier;
use basset::hub::StateResponse;

pub struct MarsMockQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    native_querier: NativeQuerier,
    cw20_querier: Cw20Querier,
    xmars_querier: XMarsQuerier,
    astroport_factory_querier: AstroportFactoryQuerier,
    astroport_pair_querier: AstroportPairQuerier,
    oracle_querier: OracleQuerier,
    staking_querier: StakingQuerier,
    vesting_querier: VestingQuerier,
    incentives_querier: IncentivesQuerier,
    basset_querier: BAssetQuerier,
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
            xmars_querier: XMarsQuerier::default(),
            astroport_factory_querier: AstroportFactoryQuerier::default(),
            astroport_pair_querier: AstroportPairQuerier::default(),
            oracle_querier: OracleQuerier::default(),
            staking_querier: StakingQuerier::default(),
            vesting_querier: VestingQuerier::default(),
            incentives_querier: IncentivesQuerier::default(),
            basset_querier: BAssetQuerier::default(),
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

    pub fn set_staking_xmars_per_mars(&mut self, xmars_per_mars: Decimal) {
        self.staking_querier.xmars_per_mars = xmars_per_mars;
        self.staking_querier.mars_per_xmars = xmars_per_mars.inv().unwrap();
    }

    pub fn set_staking_mars_per_xmars(&mut self, mars_per_xmars: Decimal) {
        self.staking_querier.mars_per_xmars = mars_per_xmars;
        self.staking_querier.xmars_per_mars = mars_per_xmars.inv().unwrap();
    }

    pub fn set_xmars_address(&mut self, address: Addr) {
        self.xmars_querier.xmars_address = address;
    }

    pub fn set_xmars_balance_at(&mut self, address: Addr, block: u64, balance: Uint128) {
        self.xmars_querier
            .balances_at
            .insert((address, block), balance);
    }

    pub fn set_xmars_total_supply_at(&mut self, block: u64, balance: Uint128) {
        self.xmars_querier.total_supplies_at.insert(block, balance);
    }

    pub fn set_vesting_address(&mut self, address: Addr) {
        self.vesting_querier.vesting_address = address;
    }

    pub fn set_vesting_voting_power_at(
        &mut self,
        address: Addr,
        block: u64,
        voting_power: Uint128,
    ) {
        self.vesting_querier
            .voting_power_at
            .insert((address, block), voting_power);
    }

    pub fn set_vesting_total_voting_power_at(&mut self, block: u64, total_voting_power: Uint128) {
        self.vesting_querier
            .total_voting_power_at
            .insert(block, total_voting_power);
    }

    pub fn set_astroport_pair(&mut self, pair_info: PairInfo) {
        let asset_infos = &pair_info.asset_infos;

        // factory
        let key = format!("{}-{}", asset_infos[0], asset_infos[1]);
        self.astroport_factory_querier
            .pairs
            .insert(key, pair_info.clone());

        // pair
        let pool_response = PoolResponse {
            assets: [
                Asset {
                    info: asset_infos[0].clone(),
                    amount: Uint128::zero(),
                },
                Asset {
                    info: asset_infos[1].clone(),
                    amount: Uint128::zero(),
                },
            ],
            total_share: Uint128::zero(),
        };
        let key = pair_info.contract_addr.to_string();
        self.astroport_pair_querier.pairs.insert(key, pool_response);
    }

    pub fn set_astroport_pair_cumulative_prices(
        &mut self,
        contract_addr: String,
        cumulative_prices: CumulativePricesResponse,
    ) {
        self.astroport_pair_querier
            .cumulative_prices
            .insert(contract_addr, cumulative_prices);
    }

    pub fn set_astroport_pair_simulation(
        &mut self,
        contract_addr: String,
        simulation: SimulationResponse,
    ) {
        self.astroport_pair_querier
            .simulations
            .insert(contract_addr, simulation);
    }

    pub fn set_incentives_address(&mut self, address: Addr) {
        self.incentives_querier.incentives_address = address;
    }

    pub fn set_unclaimed_rewards(&mut self, user_address: String, unclaimed_rewards: Uint128) {
        self.incentives_querier
            .unclaimed_rewards_at
            .insert(Addr::unchecked(user_address), unclaimed_rewards);
    }

    pub fn set_basset_state_response(&mut self, state_response: StateResponse) {
        self.basset_querier.state_response = Some(state_response);
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

                // XMars Queries
                let parse_xmars_query: StdResult<xmars_token::msg::QueryMsg> = from_binary(msg);
                if let Ok(xmars_query) = parse_xmars_query {
                    return self.xmars_querier.handle_query(&contract_addr, xmars_query);
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

                // Staking Queries
                let parse_staking_query: StdResult<staking::msg::QueryMsg> = from_binary(msg);
                if let Ok(staking_query) = parse_staking_query {
                    return self
                        .staking_querier
                        .handle_query(&contract_addr, staking_query);
                }

                // Astroport Queries
                let astroport_factory_query: StdResult<astroport::factory::QueryMsg> =
                    from_binary(msg);
                if let Ok(factory_query) = astroport_factory_query {
                    return self.astroport_factory_querier.handle_query(&factory_query);
                }

                let astroport_pair_query: StdResult<astroport::pair::QueryMsg> = from_binary(msg);
                if let Ok(pair_query) = astroport_pair_query {
                    return self
                        .astroport_pair_querier
                        .handle_query(&contract_addr, &pair_query);
                }

                // Incentives Queries
                let parse_incentives_query: StdResult<incentives::msg::QueryMsg> = from_binary(msg);
                if let Ok(incentives_query) = parse_incentives_query {
                    return self
                        .incentives_querier
                        .handle_query(&contract_addr, incentives_query);
                }

                // Vesting Queries
                let parse_vesting_query: StdResult<vesting::msg::QueryMsg> = from_binary(msg);
                if let Ok(vesting_query) = parse_vesting_query {
                    return self
                        .vesting_querier
                        .handle_query(&contract_addr, vesting_query);
                }

                // bAsset Queries
                let basset_query: StdResult<basset::hub::QueryMsg> = from_binary(msg);
                if let Ok(query) = basset_query {
                    return self.basset_querier.handle_query(&query);
                }

                panic!("[mock]: Unsupported wasm query: {:?}", msg);
            }

            _ => self.base.handle_query(request),
        }
    }
}
