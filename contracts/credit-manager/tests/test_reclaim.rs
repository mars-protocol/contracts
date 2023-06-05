use cosmwasm_std::{coin, coins, Addr, Uint128};
use mars_rover::{
    error::ContractError,
    msg::execute::Action::{Deposit, Lend, Reclaim},
};

use crate::helpers::{assert_err, get_coin, uatom_info, uosmo_info, AccountToFund, MockEnv};

pub mod helpers;

#[test]
fn only_token_owner_can_reclaim() {
    let coin_info = uosmo_info();
    let owner = Addr::unchecked("owner");
    let mut mock = MockEnv::new().build().unwrap();
    let account_id = mock.create_credit_account(&owner).unwrap();

    let another_user = Addr::unchecked("another_user");
    let res = mock.update_credit_account(
        &account_id,
        &another_user,
        vec![Reclaim(coin_info.to_action_coin(56789))],
        &[],
    );

    assert_err(
        res,
        ContractError::NotTokenOwner {
            user: another_user.into(),
            account_id,
        },
    )
}

#[test]
fn reclaiming_with_zero_lent() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().set_params(&[coin_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    // When passing some amount
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Reclaim(coin_info.to_action_coin(10))],
        &[],
    );

    assert_err(res, ContractError::NoneLent);

    // When passing no amount
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Reclaim(coin_info.to_action_coin_full_balance())],
        &[],
    );

    assert_err(res, ContractError::NoneLent);
}

#[test]
fn when_trying_to_reclaim_more_than_lent() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(coin_info.to_coin(300)), Lend(coin_info.to_coin(50))],
        &[coin_info.to_coin(300)],
    )
    .unwrap();

    // Assert account id's position
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 1);
    assert_eq!(get_coin(&coin_info.denom, &positions.deposits), coin_info.to_coin(250));

    // Assert Rover's balances
    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::new(250));

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Reclaim(coin_info.to_action_coin(500))],
        &[],
    )
    .unwrap();

    // Should reclaim only max value lent which is 50 not entire 500
    // Entire lent share should go to zero

    // Assert account id's position
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 1);
    assert_eq!(positions.lends.len(), 0);
    assert_eq!(get_coin(&coin_info.denom, &positions.deposits), coin_info.to_coin(301)); // +1 for interest from red bank

    // Assert Rover's balances
    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::new(301));
}

#[test]
fn reclaiming_less_than_entire_lent_share() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(coin_info.to_coin(300)), Lend(coin_info.to_coin(200))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    // Assert account id's position
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.lends.len(), 1);
    assert_eq!(get_coin(&coin_info.denom, &position.deposits), coin_info.to_coin(100));

    // Assert Rover's balances
    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::new(100));

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Reclaim(coin_info.to_action_coin(100))],
        &[],
    )
    .unwrap();

    // lent share should still exist but value should decrease and coin balance should increase

    // Assert account id's position
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.lends.len(), 1);
    assert_eq!(get_coin(&coin_info.denom, &position.deposits), coin_info.to_coin(200));
    assert_eq!(position.lends.first().unwrap().amount, Uint128::new(101));
    // Assert Rover's balances
    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::new(200));
}

#[test]
fn reclaiming_the_entire_lent_share() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");

    let mut mock = MockEnv::new()
        .set_params(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(coin_info.to_coin(300)), Lend(coin_info.to_coin(100))],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    // Assert account id's position
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.lends.len(), 1);
    assert_eq!(get_coin(&coin_info.denom, &position.deposits), coin_info.to_coin(200));
    assert_eq!(position.lends.first().unwrap().amount, Uint128::new(101));

    // Assert Rover's balances
    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::new(200));

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Reclaim(coin_info.to_action_coin(101))],
        &[],
    )
    .unwrap();

    // lent share should be removed

    // Assert account id's position
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.lends.len(), 0);
    assert_eq!(get_coin(&coin_info.denom, &position.deposits), coin_info.to_coin(301));

    // Assert Rover's balances
    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::new(301));
}
#[test]
fn reclaiming_multiple_assets() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();
    let user = Addr::unchecked("user");

    let mut mock = MockEnv::new()
        .set_params(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, uatom_info.denom.clone()),
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(uatom_info.to_coin(300)), Lend(uatom_info.to_coin(100))],
        &[coin(300, uatom_info.denom.clone())],
    )
    .unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(uosmo_info.to_coin(200)), Lend(uosmo_info.to_coin(100))],
        &[coin(200, uosmo_info.denom.clone())],
    )
    .unwrap();
    // Assert account id's position
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.lends.len(), 2);
    assert_eq!(get_coin(&uatom_info.denom, &position.deposits), uatom_info.to_coin(200));
    assert_eq!(get_coin(&uosmo_info.denom, &position.deposits), uosmo_info.to_coin(100));
    assert_eq!(position.lends.first().unwrap().amount, Uint128::new(101)); // +1 for interest from red bank

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Reclaim(uosmo_info.to_action_coin(101))],
        &[],
    )
    .unwrap();

    // 1 lent share should be removed and 1 should stay

    // Assert account id's position
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.lends.len(), 1);
    assert_eq!(get_coin(&uosmo_info.denom, &position.deposits), uosmo_info.to_coin(201));

    // Assert Rover's balances
    let coin = mock.query_balance(&mock.rover, &uosmo_info.denom);
    assert_eq!(coin.amount, Uint128::new(201));

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Reclaim(uatom_info.to_action_coin(101))],
        &[],
    )
    .unwrap();

    // last lent share should be removed

    // Assert account id's position
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.lends.len(), 0);
    assert_eq!(get_coin(&uatom_info.denom, &position.deposits), uatom_info.to_coin(301));

    // Assert Rover's balances
    let coin = mock.query_balance(&mock.rover, &uatom_info.denom);
    assert_eq!(coin.amount, Uint128::new(301));
}
