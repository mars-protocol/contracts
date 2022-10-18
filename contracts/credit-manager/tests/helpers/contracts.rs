use cosmwasm_std::Empty;
use cw_multi_test::{App, Contract, ContractWrapper};

pub fn mock_app() -> App {
    App::default()
}

pub fn mock_rover_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        credit_manager::contract::execute,
        credit_manager::contract::instantiate,
        credit_manager::contract::query,
    )
    .with_reply(credit_manager::contract::reply);
    Box::new(contract)
}

pub fn mock_account_nft_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        account_nft::contract::execute,
        account_nft::contract::instantiate,
        account_nft::contract::query,
    );
    Box::new(contract)
}

pub fn mock_red_bank_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mock_red_bank::contract::execute,
        mock_red_bank::contract::instantiate,
        mock_red_bank::contract::query,
    );
    Box::new(contract)
}

pub fn mock_oracle_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mock_oracle::contract::execute,
        mock_oracle::contract::instantiate,
        mock_oracle::contract::query,
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
        mock_vault::contract::execute,
        mock_vault::contract::instantiate,
        mock_vault::contract::query,
    );
    Box::new(contract)
}

pub fn mock_swapper_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        swapper_mock::contract::execute,
        swapper_mock::contract::instantiate,
        swapper_mock::contract::query,
    );
    Box::new(contract)
}
