use cosmwasm_std::Empty;
use cw_multi_test::{Contract, ContractWrapper};

pub fn mock_vault_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        mars_vault::contract::execute,
        mars_vault::contract::instantiate,
        mars_vault::contract::query,
    );
    Box::new(contract)
}
