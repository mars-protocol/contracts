use cosmwasm_std::Empty;
use cw_multi_test::{App, Contract, ContractWrapper};

pub fn mock_app() -> App {
    App::default()
}

pub fn mock_rover_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_credit_manager::contract::execute,
        mars_credit_manager::contract::instantiate,
        mars_credit_manager::contract::query,
    )
    .with_reply(mars_credit_manager::contract::reply);
    Box::new(contract)
}

pub fn mock_account_nft_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_account_nft::contract::execute,
        mars_account_nft::contract::instantiate,
        mars_account_nft::contract::query,
    );
    Box::new(contract)
}

pub fn mock_address_provider_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_address_provider::contract::execute,
        mars_address_provider::contract::instantiate,
        mars_address_provider::contract::query,
    );
    Box::new(contract)
}

pub fn mock_red_bank_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_red_bank::contract::execute,
        mars_mock_red_bank::contract::instantiate,
        mars_mock_red_bank::contract::query,
    );
    Box::new(contract)
}

pub fn mock_incentives_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_incentives::contract::execute,
        mars_mock_incentives::contract::instantiate,
        mars_mock_incentives::contract::query,
    );
    Box::new(contract)
}

pub fn mock_oracle_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_oracle::contract::execute,
        mars_mock_oracle::contract::instantiate,
        mars_mock_oracle::contract::query,
    );
    Box::new(contract)
}

pub fn mock_vault_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_vault::contract::execute,
        mars_mock_vault::contract::instantiate,
        mars_mock_vault::contract::query,
    );
    Box::new(contract)
}

pub fn mock_swapper_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_swapper_mock::contract::execute,
        mars_swapper_mock::contract::instantiate,
        mars_swapper_mock::contract::query,
    );
    Box::new(contract)
}

pub fn mock_v2_zapper_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_zapper_mock::contract::execute,
        mars_zapper_mock::contract::instantiate,
        mars_zapper_mock::contract::query,
    );
    Box::new(contract)
}

pub fn mock_health_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_rover_health::contract::execute,
        mars_rover_health::contract::instantiate,
        mars_rover_health::contract::query,
    );
    Box::new(contract)
}

pub fn mock_params_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_params::contract::execute,
        mars_params::contract::instantiate,
        mars_params::contract::query,
    );
    Box::new(contract)
}
