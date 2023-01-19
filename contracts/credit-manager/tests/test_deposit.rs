use cosmwasm_std::{coin, coins, Addr, Coin, Uint128};
use mars_rover::{
    coins::Coins,
    error::ContractError::{ExtraFundsReceived, FundsMismatch, NotTokenOwner, NotWhitelisted},
    msg::{execute::Action, query::Positions},
};

use crate::helpers::{
    assert_err, uatom_info, ujake_info, uosmo_info, AccountToFund, CoinInfo, MockEnv,
};

pub mod helpers;

#[test]
fn only_owner_of_token_can_deposit() {
    let mut mock = MockEnv::new().build().unwrap();
    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();

    let another_user = Addr::unchecked("another_user");
    let res = mock.update_credit_account(
        &account_id,
        &another_user,
        vec![Action::Deposit(coin(0, "uosmo"))],
        &[],
    );

    assert_err(
        res,
        NotTokenOwner {
            user: another_user.into(),
            account_id,
        },
    )
}

#[test]
fn deposit_nothing() {
    let coin_info = uosmo_info();

    let mut mock = MockEnv::new().allowed_coins(&[coin_info.clone()]).build().unwrap();
    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Deposit(coin_info.to_coin(0))],
        &[],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn deposit_but_no_funds() {
    let coin_info = uosmo_info();

    let mut mock = MockEnv::new().allowed_coins(&[coin_info.clone()]).build().unwrap();
    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();

    let deposit_amount = Uint128::new(234);
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Deposit(coin_info.to_coin(deposit_amount.u128()))],
        &[],
    );

    assert_err(
        res,
        FundsMismatch {
            expected: deposit_amount,
            received: Uint128::zero(),
        },
    );

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn deposit_but_not_enough_funds() {
    let coin_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Deposit(coin_info.to_coin(350))],
        &[coin(250, coin_info.denom)],
    );

    assert_err(
        res,
        FundsMismatch {
            expected: Uint128::new(350),
            received: Uint128::new(250),
        },
    );
}

#[test]
fn can_only_deposit_allowed_assets() {
    let coin_info = uosmo_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let not_allowed_coin = ujake_info().to_coin(234);

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Deposit(not_allowed_coin.clone())],
        &[coin(250, coin_info.denom)],
    );

    assert_err(res, NotWhitelisted(not_allowed_coin.denom));

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn extra_funds_received() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(300, uosmo_info.denom.clone()), coin(250, uatom_info.denom.clone())],
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let extra_funds = coin(25, uatom_info.denom);
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Deposit(uosmo_info.to_coin(234))],
        &[coin(234, uosmo_info.denom), extra_funds.clone()],
    );

    assert_err(res, ExtraFundsReceived(Coins::try_from(vec![extra_funds]).unwrap()));

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 0);
}

#[test]
fn deposit_success() {
    let coin_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let deposit_amount = Uint128::new(234);
    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::Deposit(coin_info.to_coin(deposit_amount.u128()))],
        &[Coin::new(deposit_amount.into(), coin_info.denom.clone())],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    let assets_res = res.deposits.first().unwrap();
    assert_eq!(res.deposits.len(), 1);
    assert_eq!(assets_res.amount, deposit_amount);
    assert_eq!(assets_res.denom, coin_info.denom);

    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, deposit_amount)
}

#[test]
fn multiple_deposit_actions() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(300, uosmo_info.denom.clone()), coin(50, uatom_info.denom.clone())],
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let uosmo_amount = Uint128::new(234);
    let uatom_amount = Uint128::new(25);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(uosmo_info.to_coin(uosmo_amount.u128())),
            Action::Deposit(uatom_info.to_coin(uatom_amount.u128())),
        ],
        &[coin(234, uosmo_info.denom.clone()), coin(25, uatom_info.denom.clone())],
    )
    .unwrap();

    let res = mock.query_positions(&account_id);
    assert_eq!(res.deposits.len(), 2);
    assert_present(&res, &uosmo_info, uosmo_amount);
    assert_present(&res, &uatom_info, uatom_amount);

    let coin = mock.query_balance(&mock.rover, &uosmo_info.denom);
    assert_eq!(coin.amount, uosmo_amount);

    let coin = mock.query_balance(&mock.rover, &uatom_info.denom);
    assert_eq!(coin.amount, uatom_amount);
}

fn assert_present(res: &Positions, coin: &CoinInfo, amount: Uint128) {
    res.deposits.iter().find(|item| item.denom == coin.denom && item.amount == amount).unwrap();
}
