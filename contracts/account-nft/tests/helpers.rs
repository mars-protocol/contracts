use anyhow::Result as AnyResult;
use cosmwasm_std::Addr;
use cw721_base::InstantiateMsg;
use cw_multi_test::{AppResponse, BasicApp, ContractWrapper, Executor};

use mars_account_nft::contract::{execute, instantiate, query};
use mars_account_nft::msg::ExecuteMsg as ExtendedExecuteMsg;

pub fn instantiate_mock_nft_contract(app: &mut BasicApp, owner: &Addr) -> Addr {
    let contract = Box::new(ContractWrapper::new(execute, instantiate, query));
    let code_id = app.store_code(contract);

    app.instantiate_contract(
        code_id,
        owner.clone(),
        &InstantiateMsg {
            name: "mock_nft".to_string(),
            symbol: "MOCK".to_string(),
            minter: owner.to_string(),
        },
        &[],
        "mock-account-nft",
        None,
    )
    .unwrap()
}

pub fn mint_action(
    app: &mut BasicApp,
    sender: &Addr,
    contract_addr: &Addr,
    token_owner: &Addr,
) -> AnyResult<AppResponse> {
    app.execute_contract(
        sender.clone(),
        contract_addr.clone(),
        &ExtendedExecuteMsg::Mint {
            user: token_owner.into(),
        },
        &[],
    )
}

pub fn burn_action(
    app: &mut BasicApp,
    sender: &Addr,
    contract_addr: &Addr,
    token_id: &str,
) -> AnyResult<AppResponse> {
    app.execute_contract(
        sender.clone(),
        contract_addr.clone(),
        &ExtendedExecuteMsg::Burn {
            token_id: token_id.to_string(),
        },
        &[],
    )
}
