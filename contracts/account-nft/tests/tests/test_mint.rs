use cosmwasm_std::Addr;
use cw721::OwnerOfResponse;
use cw721_base::{ContractError::Ownership, OwnershipError::NotOwner};
use cw_multi_test::Executor;
use mars_account_nft::error::{ContractError, ContractError::BaseError};
use mars_types::{
    account_nft::{ExecuteMsg, QueryMsg::OwnerOf},
    health::AccountKind,
};
use proptest::prelude::*;

use super::helpers::{below_max_for_burn, MockEnv};

#[test]
fn id_incrementer() {
    let mut mock = MockEnv::new().build().unwrap();
    mock.assert_next_id("1");

    let user_1 = Addr::unchecked("user_1");
    let token_id = mock.mint(&user_1).unwrap();
    assert_eq!(token_id, "1");
    mock.assert_owner_is_correct(&user_1, &token_id);
    mock.assert_next_id("2");

    let user_2 = Addr::unchecked("user_2");
    let token_id = mock.mint(&user_2).unwrap();
    assert_eq!(token_id, "2");
    mock.assert_owner_is_correct(&user_2, &token_id);
    mock.assert_next_id("3");

    let user_3 = Addr::unchecked("user_3");
    let token_id = mock.mint(&user_3).unwrap();
    assert_eq!(token_id, "3");
    mock.assert_owner_is_correct(&user_3, &token_id);
    mock.assert_next_id("4");
}

#[test]
fn id_incrementer_works_despite_burns() {
    let mut mock = MockEnv::new().build().unwrap();
    mock.assert_next_id("1");

    let user = Addr::unchecked("user");
    let token_id_1 = mock.mint(&user).unwrap();
    assert_eq!(token_id_1, "1");
    mock.assert_next_id("2");

    let token_id_2 = mock.mint(&user).unwrap();
    assert_eq!(token_id_2, "2");
    mock.assert_next_id("3");

    mock.set_health_response(&user, &token_id_1, AccountKind::Default, &below_max_for_burn());
    mock.burn(&user, &token_id_1).unwrap();
    mock.set_health_response(&user, &token_id_2, AccountKind::Default, &below_max_for_burn());
    mock.burn(&user, &token_id_2).unwrap();

    mock.assert_next_id("3");
    let token_id_3 = mock.mint(&user).unwrap();
    assert_eq!(token_id_3, "3");
    mock.assert_owner_is_correct(&user, &token_id_3);
    mock.assert_next_id("4");
}

#[test]
fn only_minter_can_mint() {
    let mut mock = MockEnv::new().set_minter("mr_minter").build().unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.app.execute_contract(
        bad_guy.clone(),
        mock.nft_contract.clone(),
        &ExecuteMsg::Mint {
            user: bad_guy.into(),
            token_id: None,
        },
        &[],
    );
    let err: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(err, BaseError(Ownership(NotOwner)))
}

#[test]
fn normal_base_cw721_actions_can_still_be_taken() {
    let mut mock = MockEnv::new().build().unwrap();

    let rover_user_a = Addr::unchecked("rover_user_a");
    let token_id = mock.mint(&rover_user_a).unwrap();

    let rover_user_b = Addr::unchecked("rover_user_b");
    let transfer_msg: ExecuteMsg = ExecuteMsg::TransferNft {
        token_id: token_id.clone(),
        recipient: rover_user_b.clone().into(),
    };
    mock.app.execute_contract(rover_user_a, mock.nft_contract.clone(), &transfer_msg, &[]).unwrap();

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

#[test]
fn invalid_custom_token_id_length() {
    let mut mock = MockEnv::new().build().unwrap();
    mock.assert_next_id("1");

    let user = Addr::unchecked("user_abc");

    let res = mock.mint_with_custom_token_id(&user, Some("abc".to_string()));
    let err: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        err,
        ContractError::InvalidTokenId {
            reason: "token_id length should be between 4 and 15 chars".to_string()
        }
    );
    mock.assert_next_id("1");

    let res = mock.mint_with_custom_token_id(&user, Some("abcdefghijklmnop".to_string()));
    let err: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        err,
        ContractError::InvalidTokenId {
            reason: "token_id length should be between 4 and 15 chars".to_string()
        }
    );
    mock.assert_next_id("1");
}

#[test]
fn custom_token_id_can_not_be_same_as_automatically_generated() {
    let mut mock = MockEnv::new().build().unwrap();
    mock.assert_next_id("1");

    let user = Addr::unchecked("user_abc");

    let res = mock.mint_with_custom_token_id(&user, Some("12345".to_string()));
    let err: ContractError = res.unwrap_err().downcast().unwrap();
    assert_eq!(
        err,
        ContractError::InvalidTokenId {
            reason: "token_id should contain at least one letter".to_string()
        }
    );
    mock.assert_next_id("1");
}

proptest! {
    #[test]
    fn invalid_custom_token_id_characters(token_id in "[!@#$%^&*()-+]{4,15}") {
        let mut mock = MockEnv::new().build().unwrap();
        mock.assert_next_id("1");

        let user = Addr::unchecked("user_abc");
        let res = mock.mint_with_custom_token_id(&user, Some(token_id));
        let err: ContractError = res.unwrap_err().downcast().unwrap();
        prop_assert_eq!(err, ContractError::InvalidTokenId { reason: "token_id can contain only letters, numbers, and underscores".to_string() });
        mock.assert_next_id("1");
    }

    /// The regex pattern ensures that the string:
    /// - starts with three characters (letters, digits, or underscores),
    /// - contains at least one letter,
    /// - can have up to 11 additional characters (letters, digits, or underscores) to fulfill the length requirement.
    #[test]
    fn valid_custom_token_id(token_id in "[a-zA-Z0-9_]{3}[a-zA-Z][a-zA-Z0-9_]{0,11}") {
        let mut mock = MockEnv::new().build().unwrap();
        mock.assert_next_id("1");

        let user = Addr::unchecked("user_abc");
        let saved_token_id = mock.mint_with_custom_token_id(&user, Some(token_id.clone())).unwrap();
        assert_eq!(saved_token_id, token_id);
        mock.assert_next_id("1");
    }
}
