use cosmwasm_std::{coin, Addr};

use helpers::assert_err;
use rover::error::ContractError;
use rover::error::ContractError::NotTokenOwner;
use rover::msg::execute::CallbackMsg;

use crate::helpers::MockEnv;

pub mod helpers;

#[test]
fn test_dispatch_only_allowed_for_token_owner() {
    let mut mock = MockEnv::new().build().unwrap();
    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_credit_account(&account_id, &bad_guy, vec![], &[]);

    assert_err(
        res,
        NotTokenOwner {
            user: bad_guy.into(),
            account_id,
        },
    )
}

#[test]
fn test_nothing_happens_if_no_actions_are_passed() {
    let mut mock = MockEnv::new().build().unwrap();
    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.query_position(&account_id);
    assert_eq!(res.coins.len(), 0);

    mock.update_credit_account(&account_id, &user, vec![], &[])
        .unwrap();

    let res = mock.query_position(&account_id);
    assert_eq!(res.coins.len(), 0);
}

#[test]
fn test_only_rover_can_execute_callbacks() {
    let mut mock = MockEnv::new().build().unwrap();
    let external_user = Addr::unchecked("external_user");

    let res = mock.execute_callback(
        &external_user,
        CallbackMsg::Borrow {
            account_id: "1234".to_string(),
            coin: coin(1000, "uatom"),
        },
    );
    assert_err(res, ContractError::ExternalInvocation);
}
