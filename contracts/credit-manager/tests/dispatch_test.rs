extern crate core;

use cosmwasm_std::Addr;
use cw_multi_test::Executor;

use rover::error::ContractError::NotTokenOwner;
use rover::msg::ExecuteMsg::UpdateCreditAccount;

use helpers::{
    assert_err, get_token_id, mock_app, mock_create_credit_account, query_position,
    setup_credit_manager,
};

pub mod helpers;

#[test]
fn test_dispatch_only_allowed_for_token_owner() {
    let mut app = mock_app();
    let owner = Addr::unchecked("owner");
    let mock = setup_credit_manager(&mut app, &owner, vec![], vec![]);

    let user = Addr::unchecked("user");
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let bad_guy = Addr::unchecked("bad_guy");
    let res = app.execute_contract(
        bad_guy.clone(),
        mock.credit_manager.clone(),
        &UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![],
        },
        &[],
    );

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
    let mut app = mock_app();
    let owner = Addr::unchecked("owner");
    let mock = setup_credit_manager(&mut app, &owner, vec![], vec![]);

    let user = Addr::unchecked("user");
    let res = mock_create_credit_account(&mut app, &mock.credit_manager, &user).unwrap();
    let token_id = get_token_id(res);

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.coin_assets.len(), 0);

    app.execute_contract(
        user.clone(),
        mock.credit_manager.clone(),
        &UpdateCreditAccount {
            token_id: token_id.clone(),
            actions: vec![],
        },
        &[],
    )
    .unwrap();

    let res = query_position(&app, &mock.credit_manager, &token_id);
    assert_eq!(res.coin_assets.len(), 0);
}
