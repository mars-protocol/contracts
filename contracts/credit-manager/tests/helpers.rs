use cosmwasm_std::Empty;
use cw_multi_test::{App, Contract, ContractWrapper};

use account_nft::contract::{
    execute as cw721Execute, instantiate as cw721Instantiate, query as cw721Query,
};
use credit_manager::contract::{execute, instantiate, query};

pub fn mock_app() -> App {
    App::default()
}

pub fn mock_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_account_nft_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(cw721Execute, cw721Instantiate, cw721Query);
    Box::new(contract)
}
