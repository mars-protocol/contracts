use std::fmt::Error;

use cosmwasm_std::Addr;
use cw721::OwnerOfResponse;
use cw721_base::ContractError::Unauthorized;
use cw_multi_test::Executor;
use mars_account_nft::{
    error::{ContractError, ContractError::BaseError},
    msg::{ExecuteMsg as ExtendedExecuteMsg, QueryMsg::OwnerOf},
};

use crate::helpers::{below_max_for_burn, MockEnv};

pub mod helpers;

#[test]
fn test_id_incrementer() {
    let mut mock = MockEnv::new().build().unwrap();

    let user_1 = Addr::unchecked("user_1");
    let token_id = mock.mint(&user_1).unwrap();
    assert_eq!(token_id, "1");
    mock.assert_owner_is_correct(&user_1, &token_id);

    let user_2 = Addr::unchecked("user_2");
    let token_id = mock.mint(&user_2).unwrap();
    assert_eq!(token_id, "2");
    mock.assert_owner_is_correct(&user_2, &token_id);

    let user_3 = Addr::unchecked("user_3");
    let token_id = mock.mint(&user_3).unwrap();
    assert_eq!(token_id, "3");
    mock.assert_owner_is_correct(&user_3, &token_id);
}

#[test]
fn test_id_incrementer_works_despite_burns() {
    let mut mock = MockEnv::new().assign_minter_to_cm().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id_1 = mock.mint(&user).unwrap();
    assert_eq!(token_id_1, "1");

    let token_id_2 = mock.mint(&user).unwrap();
    assert_eq!(token_id_2, "2");

    mock.set_health_response(&user, &token_id_1, &below_max_for_burn());
    mock.burn(&user, &token_id_1).unwrap();
    mock.set_health_response(&user, &token_id_2, &below_max_for_burn());
    mock.burn(&user, &token_id_2).unwrap();

    let token_id_3 = mock.mint(&user).unwrap();
    assert_eq!(token_id_3, "3");
    mock.assert_owner_is_correct(&user, &token_id_3);
}

#[test]
fn test_only_minter_can_mint() {
    let mut mock = MockEnv::new().set_minter("mr_minter").build().unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.app.execute_contract(
        bad_guy.clone(),
        mock.nft_contract.clone(),
        &ExtendedExecuteMsg::Mint {
            user: bad_guy.into(),
        },
        &[],
    );
    let err: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(err, BaseError(Unauthorized {}))
}

#[test]
fn test_only_token_owner_can_burn() {
    let mut mock = MockEnv::new().assign_minter_to_cm().build().unwrap();

    let user = Addr::unchecked("user");
    let token_id = mock.mint(&user).unwrap();
    mock.set_health_response(&user, &token_id, &below_max_for_burn());

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.burn(&bad_guy, &token_id);
    let err: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(err, BaseError(Unauthorized {}));

    mock.burn(&user, &token_id).unwrap();
}

#[test]
fn test_normal_base_cw721_actions_can_still_be_taken() {
    let mut mock = MockEnv::new().build().unwrap();

    let rover_user_a = Addr::unchecked("rover_user_a");
    let token_id = mock.mint(&rover_user_a).unwrap();

    let rover_user_b = Addr::unchecked("rover_user_b");
    let transfer_msg: ExtendedExecuteMsg = ExtendedExecuteMsg::TransferNft {
        token_id: token_id.clone(),
        recipient: rover_user_b.clone().into(),
    };
    mock.app
        .execute_contract(rover_user_a, mock.nft_contract.clone(), &transfer_msg, &[])
        .map_err(|_| Error::default())
        .unwrap();

    let res: OwnerOfResponse = mock
        .app
        .wrap()
        .query_wasm_smart(
            mock.nft_contract,
            &OwnerOf {
                token_id,
                include_expired: None,
            },
        )
        .unwrap();
    assert_eq!(res.owner, rover_user_b.to_string())
}
