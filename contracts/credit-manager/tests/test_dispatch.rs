use cosmwasm_std::{Addr, Coin, Uint128};

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
    let token_id = mock.create_credit_account(&user).unwrap();

    let bad_guy = Addr::unchecked("bad_guy");
    let res = mock.update_credit_account(&token_id, &bad_guy, vec![], &[]);

    assert_err(
        res,
        NotTokenOwner {
            user: bad_guy.into(),
            token_id,
        },
    )
}

#[test]
fn test_nothing_happens_if_no_actions_are_passed() {
    let mut mock = MockEnv::new().build().unwrap();
    let user = Addr::unchecked("user");
    let token_id = mock.create_credit_account(&user).unwrap();

    let res = mock.query_position(&token_id);
    assert_eq!(res.coins.len(), 0);

    mock.update_credit_account(&token_id, &user, vec![], &[])
        .unwrap();

    let res = mock.query_position(&token_id);
    assert_eq!(res.coins.len(), 0);
}

#[test]
fn test_only_rover_can_execute_callbacks() {
    let mut mock = MockEnv::new().build().unwrap();
    let external_user = Addr::unchecked("external_user");

    let res = mock.execute_callback(
        &external_user,
        CallbackMsg::Borrow {
            token_id: "1234".to_string(),
            coin: Coin {
                denom: "uatom".to_string(),
                amount: Uint128::new(1000u128),
            },
        },
    );
    assert_err(res, ContractError::ExternalInvocation);
}
