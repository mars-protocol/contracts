use std::fmt::Error;

use cosmwasm_std::{Addr, Empty};
use cw721::OwnerOfResponse;
use cw721_base::QueryMsg;
use cw_multi_test::{App, AppResponse, BasicApp, Executor};

use account_nft::msg::ExecuteMsg as ExtendedExecuteMsg;

use crate::helpers::{instantiate_mock_nft_contract, mint_action};

pub mod helpers;

#[test]
fn test_id_incrementer() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let contract_addr = instantiate_mock_nft_contract(&mut app, &owner);

    let user_1 = Addr::unchecked("user_1");
    let res = mint_action(&mut app, &owner, &contract_addr, &user_1).unwrap();
    let token_id = get_token_id(res);
    assert_eq!(token_id, "1");
    assert_owner_is_correct(&mut app, &contract_addr, &user_1, &token_id);

    let user_2 = Addr::unchecked("user_2");
    let res = mint_action(&mut app, &owner, &contract_addr, &user_2).unwrap();
    let token_id = get_token_id(res);
    assert_eq!(token_id, "2");
    assert_owner_is_correct(&mut app, &contract_addr, &user_2, &token_id);

    let user_3 = Addr::unchecked("user_3");
    let res = mint_action(&mut app, &owner, &contract_addr, &user_3).unwrap();
    let token_id = get_token_id(res);
    assert_eq!(token_id, "3");
    assert_owner_is_correct(&mut app, &contract_addr, &user_3, &token_id);
}

#[test]
fn test_only_owner_can_mint() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let contract_addr = instantiate_mock_nft_contract(&mut app, &owner);

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mint_action(&mut app, &bad_guy, &contract_addr, &bad_guy);
    if res.is_ok() {
        panic!("Unauthorized access to minting function");
    }
}

#[test]
fn test_normal_base_cw721_actions_can_still_be_taken() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let contract_addr = instantiate_mock_nft_contract(&mut app, &owner);

    let rover_user = Addr::unchecked("rover_user");
    let res = mint_action(&mut app, &owner, &contract_addr, &rover_user).unwrap();
    let token_id = get_token_id(res);

    let burn_msg: ExtendedExecuteMsg = ExtendedExecuteMsg::Burn { token_id };
    app.execute_contract(rover_user, contract_addr.clone(), &burn_msg, &[])
        .map_err(|_| Error::default())
        .unwrap();
}

// Double checking ownership by querying NFT account-nft for correct owner
fn assert_owner_is_correct(app: &mut BasicApp, contract_addr: &Addr, user: &Addr, token_id: &str) {
    let owner_res: OwnerOfResponse = app
        .wrap()
        .query_wasm_smart(
            contract_addr,
            &QueryMsg::<Empty>::OwnerOf {
                token_id: token_id.to_string(),
                include_expired: None,
            },
        )
        .unwrap();

    assert_eq!(user.to_string(), owner_res.owner)
}

fn get_token_id(res: AppResponse) -> String {
    let attr: Vec<&str> = res
        .events
        .iter()
        .flat_map(|event| &event.attributes)
        .filter(|attr| attr.key == "token_id")
        .map(|attr| attr.value.as_str())
        .collect();

    assert_eq!(attr.len(), 1);
    attr.first().unwrap().to_string()
}
