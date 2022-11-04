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

pub fn mock_red_bank_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_red_bank::contract::execute,
        mars_mock_red_bank::contract::instantiate,
        mars_mock_red_bank::contract::query,
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

pub fn mock_oracle_adapter_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_oracle_adapter::contract::execute,
        mars_oracle_adapter::contract::instantiate,
        mars_oracle_adapter::contract::query,
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

pub fn mock_zapper_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_zapper::contract::execute,
        mars_mock_zapper::contract::instantiate,
        mars_mock_zapper::contract::query,
    );
    Box::new(contract)
}
