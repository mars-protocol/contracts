use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Empty};
use cw20::Cw20Coin;
use cw20_base::contract::{
    execute as cw20Execute, instantiate as cw20Instantiate, query as cw20Query,
};
use cw20_base::msg::InstantiateMsg as cw20InstantiateMsg;
use cw721_base::InstantiateMsg as NftInstantiateMsg;
use cw_asset::AssetInfoUnchecked;
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};

use account_nft::contract::{
    execute as cw721Execute, instantiate as cw721Instantiate, query as cw721Query,
};
use account_nft::msg::ExecuteMsg as NftExecuteMsg;
use credit_manager::contract::{execute, instantiate, query};
use mock_red_bank::contract::{
    execute as redBankExecute, instantiate as redBankInstantiate, query as redBankQuery,
};
use rover::adapters::RedBankBase;
use rover::error::ContractError;
use rover::msg::execute::ExecuteMsg::{CreateCreditAccount, UpdateConfig};
use rover::msg::query::{ConfigResponse, PositionResponse, QueryMsg};
use rover::msg::InstantiateMsg;

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

pub fn mock_cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(cw20Execute, cw20Instantiate, cw20Query);
    Box::new(contract)
}

pub fn mock_red_bank_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(redBankExecute, redBankInstantiate, redBankQuery);
    Box::new(contract)
}

pub fn mock_create_credit_account(
    app: &mut App,
    manager_contract_addr: &Addr,
    user: &Addr,
) -> AnyResult<AppResponse> {
    app.execute_contract(
        user.clone(),
        manager_contract_addr.clone(),
        &CreateCreditAccount {},
        &[],
    )
}

pub fn deploy_mock_cw20(app: &mut App, symbol: &str, initial_balances: Vec<Cw20Coin>) -> Addr {
    let code_id = app.store_code(mock_cw20_contract());
    app.instantiate_contract(
        code_id,
        Addr::unchecked("cw20-instantiator"),
        &cw20InstantiateMsg {
            name: format!("Token: {}", symbol.clone()),
            symbol: symbol.to_string(),
            decimals: 9,
            initial_balances,
            mint: None,
            marketing: None,
        },
        &[],
        "mock-cw20",
        None,
    )
    .unwrap()
}

pub fn transfer_nft_contract_ownership(
    app: &mut App,
    owner: &Addr,
    nft_contract_addr: &Addr,
    manager_contract_addr: &Addr,
) {
    let proposal_msg: NftExecuteMsg = NftExecuteMsg::ProposeNewOwner {
        new_owner: manager_contract_addr.to_string(),
    };
    app.execute_contract(owner.clone(), nft_contract_addr.clone(), &proposal_msg, &[])
        .unwrap();

    app.execute_contract(
        owner.clone(),
        manager_contract_addr.clone(),
        &UpdateConfig {
            account_nft: Some(nft_contract_addr.to_string()),
            owner: None,
            red_bank: None,
        },
        &[],
    )
    .unwrap();
}

pub fn setup_red_bank(app: &mut App) -> Addr {
    let contract_code_id = app.store_code(mock_red_bank_contract());
    app.instantiate_contract(
        contract_code_id,
        Addr::unchecked("red_bank_contract_owner"),
        &Empty {},
        &[],
        "mock-red-bank",
        None,
    )
    .unwrap()
}

pub fn setup_nft_contract(app: &mut App, owner: &Addr, manager_contract_addr: &Addr) -> Addr {
    let nft_contract_code_id = app.store_code(mock_account_nft_contract());
    let nft_contract_addr = app
        .instantiate_contract(
            nft_contract_code_id,
            owner.clone(),
            &NftInstantiateMsg {
                name: "Rover Credit Account".to_string(),
                symbol: "RCA".to_string(),
                minter: owner.to_string(),
            },
            &[],
            "manager-mock-account-nft",
            None,
        )
        .unwrap();

    transfer_nft_contract_ownership(app, owner, &nft_contract_addr, &manager_contract_addr);
    nft_contract_addr
}

pub fn setup_credit_manager(
    mut app: &mut App,
    owner: &Addr,
    allowed_assets: Vec<AssetInfoUnchecked>,
) -> Addr {
    let credit_manager_code_id = app.store_code(mock_contract());
    let red_bank_addr = setup_red_bank(app);
    let manager_initiate_msg = InstantiateMsg {
        owner: owner.to_string(),
        allowed_vaults: vec![],
        allowed_assets,
        red_bank: RedBankBase {
            contract_addr: red_bank_addr.to_string(),
        },
    };

    let manager_contract_addr = app
        .instantiate_contract(
            credit_manager_code_id,
            owner.clone(),
            &manager_initiate_msg,
            &[],
            "manager-mock",
            None,
        )
        .unwrap();

    setup_nft_contract(&mut app, &owner, &manager_contract_addr);
    manager_contract_addr
}

pub fn get_token_id(res: AppResponse) -> String {
    let attr: Vec<&String> = res
        .events
        .iter()
        .flat_map(|event| &event.attributes)
        .filter(|attr| attr.key == "token_id")
        .map(|attr| &attr.value)
        .collect();

    assert_eq!(attr.len(), 1);
    attr.first().unwrap().to_string()
}

pub fn query_position(
    app: &App,
    manager_contract_addr: &Addr,
    token_id: &String,
) -> PositionResponse {
    app.wrap()
        .query_wasm_smart(
            manager_contract_addr.clone(),
            &QueryMsg::Position {
                token_id: token_id.clone(),
            },
        )
        .unwrap()
}

pub fn assert_err(res: AnyResult<AppResponse>, err: ContractError) {
    match res {
        Ok(_) => panic!("Result was not an error"),
        Err(generic_err) => {
            let contract_err: ContractError = generic_err.downcast().unwrap();
            assert_eq!(contract_err, err);
        }
    }
}

pub fn query_config(app: &mut App, contract_addr: &Addr) -> ConfigResponse {
    app.wrap()
        .query_wasm_smart(contract_addr.clone(), &QueryMsg::Config {})
        .unwrap()
}
