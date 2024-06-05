use cosmwasm_std::Empty;
use cw_multi_test::{App, Contract, ContractWrapper};

pub fn mock_app() -> App {
    App::default()
}

pub fn mock_address_provider_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_address_provider::contract::execute,
        mars_address_provider::contract::instantiate,
        mars_address_provider::contract::query,
    );
    Box::new(contract)
}

pub fn mock_astroport_incentives() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_astroport_incentives::contract::execute,
        mars_mock_astroport_incentives::contract::instantiate,
        mars_mock_astroport_incentives::contract::query,
    );
    Box::new(contract)
}

pub fn mock_incentives_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_incentives::contract::execute,
        mars_incentives::contract::instantiate,
        mars_incentives::contract::query,
    );
    Box::new(contract)
}

pub fn mock_oracle_osmosis_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_oracle_osmosis::contract::entry::execute,
        mars_oracle_osmosis::contract::entry::instantiate,
        mars_oracle_osmosis::contract::entry::query,
    );
    Box::new(contract)
}

pub fn mock_red_bank_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_red_bank::contract::execute,
        mars_red_bank::contract::instantiate,
        mars_red_bank::contract::query,
    );
    Box::new(contract)
}

pub fn mock_rewards_collector_osmosis_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_rewards_collector_osmosis::entry::execute,
        mars_rewards_collector_osmosis::entry::instantiate,
        mars_rewards_collector_osmosis::entry::query,
    );
    Box::new(contract)
}

pub fn mock_params_osmosis_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_params::contract::execute,
        mars_params::contract::instantiate,
        mars_params::contract::query,
    );
    Box::new(contract)
}

pub fn mock_pyth_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_pyth::contract::execute,
        mars_mock_pyth::contract::instantiate,
        mars_mock_pyth::contract::query,
    );
    Box::new(contract)
}
