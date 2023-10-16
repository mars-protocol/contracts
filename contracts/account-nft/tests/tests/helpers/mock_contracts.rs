use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_nft_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_account_nft::contract::execute,
        mars_account_nft::contract::instantiate,
        mars_account_nft::contract::query,
    );
    Box::new(contract)
}

pub fn mock_health_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_rover_health::contract::execute,
        mars_mock_rover_health::contract::instantiate,
        mars_mock_rover_health::contract::query,
    );
    Box::new(contract)
}

pub fn mock_credit_manager_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_mock_credit_manager::contract::execute,
        mars_mock_credit_manager::contract::instantiate,
        mars_mock_credit_manager::contract::query,
    );
    Box::new(contract)
}
