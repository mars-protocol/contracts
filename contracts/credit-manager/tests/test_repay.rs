use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use credit_manager::borrow::DEFAULT_DEBT_SHARES_PER_COIN_BORROWED;
use std::ops::{Add, Mul, Sub};

use rover::error::ContractError;
use rover::msg::execute::Action::{Borrow, Deposit, Repay};

use crate::helpers::{assert_err, AccountToFund, CoinInfo, MockEnv, DEFAULT_RED_BANK_COIN_BALANCE};

pub mod helpers;

#[test]
fn test_only_token_owner_can_repay() {
    let coin_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };
    let owner = Addr::unchecked("owner");
    let mut mock = MockEnv::new().build().unwrap();
    let token_id = mock.create_credit_account(&owner).unwrap();

    let another_user = Addr::unchecked("another_user");
    let res = mock.update_credit_account(
        &token_id,
        &another_user,
        vec![Repay(coin_info.to_coin(Uint128::new(12312u128)))],
        &[],
    );

    assert_err(
        res,
        ContractError::NotTokenOwner {
            user: another_user.into(),
            token_id,
        },
    )
}

#[test]
fn test_can_only_repay_what_is_whitelisted() {
    let coin_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().allowed_coins(&[coin_info]).build().unwrap();
    let token_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &token_id,
        &user,
        vec![Repay(Coin {
            denom: "usomething".to_string(),
            amount: Uint128::from(234u128),
        })],
        &[],
    );

    assert_err(
        res,
        ContractError::NotWhitelisted(String::from("usomething")),
    )
}

#[test]
fn test_repaying_zero_raises() {
    let coin_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[coin_info.clone()])
        .build()
        .unwrap();
    let token_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &token_id,
        &user,
        vec![Repay(coin_info.to_coin(Uint128::zero()))],
        &[],
    );

    assert_err(res, ContractError::NoAmount)
}

#[test]
fn test_raises_when_repaying_what_is_not_owed() {
    let uosmo_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };

    let uatom_info = CoinInfo {
        denom: "atom".to_string(),
        price: Decimal::from_atomics(9u128, 0).unwrap(),
        max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(85u128, 2).unwrap(),
    };

    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");

    let mut mock = MockEnv::new()
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: user_a.clone(),
            funds: vec![Coin::new(300u128, uatom_info.denom.clone())],
        })
        .fund_account(AccountToFund {
            addr: user_b.clone(),
            funds: vec![Coin::new(100u128, uatom_info.denom.clone())],
        })
        .build()
        .unwrap();

    let token_id_a = mock.create_credit_account(&user_a).unwrap();
    let token_id_b = mock.create_credit_account(&user_b).unwrap();

    // Seeding uatom with existing total debt shares from another user
    mock.update_credit_account(
        &token_id_b,
        &user_b,
        vec![
            Deposit(uatom_info.to_coin(Uint128::from(100u128))),
            Borrow(uatom_info.to_coin(Uint128::from(12u128))),
        ],
        &[uatom_info.to_coin(Uint128::from(100u128))],
    )
    .unwrap();

    let res = mock.update_credit_account(
        &token_id_a,
        &user_a,
        vec![
            Deposit(uatom_info.to_coin(Uint128::from(300u128))),
            Borrow(uosmo_info.to_coin(Uint128::from(42u128))),
            Repay(uatom_info.to_coin(Uint128::from(42u128))),
        ],
        &[uatom_info.to_coin(Uint128::from(300u128))],
    );

    assert_err(res, ContractError::NoDebt)
}

// TODO: After withdraw is implemented, complete this test
// Should do a deposit, borrow another denom, withdraw some
// and then attempt to repay with not enough in assets
#[test]
fn test_raises_when_not_enough_assets_to_repay() {}

#[test]
fn test_successful_repay() {
    let coin_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };

    let user = Addr::unchecked("user");

    let mut mock = MockEnv::new()
        .allowed_coins(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![Coin::new(300u128, coin_info.denom.clone())],
        })
        .build()
        .unwrap();

    let token_id = mock.create_credit_account(&user).unwrap();

    let position = mock.query_position(&token_id);
    assert_eq!(position.coins.len(), 0);
    assert_eq!(position.debt_shares.len(), 0);

    mock.update_credit_account(
        &token_id,
        &user,
        vec![
            Deposit(coin_info.to_coin(Uint128::from(300u128))),
            Borrow(coin_info.to_coin(Uint128::from(50u128))),
        ],
        &[Coin::new(300u128, coin_info.denom.clone())],
    )
    .unwrap();

    let interim_red_bank_debt = mock.query_red_bank_debt(&coin_info.denom);

    mock.update_credit_account(
        &token_id,
        &user,
        vec![Repay(coin_info.to_coin(Uint128::from(20u128)))],
        &[],
    )
    .unwrap();

    let position = mock.query_position(&token_id);
    assert_eq!(position.coins.len(), 1);
    let asset_res = position.coins.first().unwrap();
    let expected_net_asset_amount = Uint128::from(330u128); // Deposit + Borrow - Repay
    assert_eq!(asset_res.amount, expected_net_asset_amount);

    let debt_shares_res = position.debt_shares.first().unwrap();
    assert_eq!(position.debt_shares.len(), 1);
    assert_eq!(debt_shares_res.denom, coin_info.denom);

    let former_total_debt_shares = Uint128::from(50u128).mul(DEFAULT_DEBT_SHARES_PER_COIN_BORROWED);
    let debt_shares_paid = former_total_debt_shares
        .multiply_ratio(Uint128::from(20u128), interim_red_bank_debt.amount);
    let new_total_debt_shares = former_total_debt_shares.sub(debt_shares_paid);
    assert_eq!(debt_shares_res.shares, new_total_debt_shares);

    let res = mock.query_total_debt_shares(&coin_info.denom);
    assert_eq!(res.shares, new_total_debt_shares);

    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::from(330u128));

    let config = mock.query_config();
    let red_bank_addr = Addr::unchecked(config.red_bank);
    let coin = mock.query_balance(&red_bank_addr, &coin_info.denom);
    assert_eq!(
        coin.amount,
        DEFAULT_RED_BANK_COIN_BALANCE.sub(Uint128::from(30u128))
    );

    mock.update_credit_account(
        &token_id,
        &user,
        vec![Repay(coin_info.to_coin(Uint128::from(31u128)))], // Interest accrued paid back as well
        &[],
    )
    .unwrap();

    let position = mock.query_position(&token_id);
    assert_eq!(position.coins.len(), 1);
    let asset_res = position.coins.first().unwrap();
    let expected_net_asset_amount = Uint128::from(299u128); // Deposit + Borrow - full repay - interest
    assert_eq!(asset_res.amount, expected_net_asset_amount);

    // Full debt repaid and purged from storage
    assert_eq!(position.debt_shares.len(), 0);

    let res = mock.query_total_debt_shares(&coin_info.denom);
    assert_eq!(res.shares, Uint128::zero());

    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::from(299u128));
    let coin = mock.query_balance(&red_bank_addr, &coin_info.denom);
    assert_eq!(
        coin.amount,
        DEFAULT_RED_BANK_COIN_BALANCE.add(Uint128::from(1u128))
    );
}

#[test]
fn test_pays_max_debt_when_attempting_to_repay_more_than_owed() {
    let coin_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(25u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
    };

    let user = Addr::unchecked("user");

    let mut mock = MockEnv::new()
        .allowed_coins(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![Coin::new(300u128, coin_info.denom.clone())],
        })
        .build()
        .unwrap();

    let token_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &token_id,
        &user,
        vec![
            Deposit(coin_info.to_coin(Uint128::from(300u128))),
            Borrow(coin_info.to_coin(Uint128::from(50u128))),
            Repay(coin_info.to_coin(Uint128::from(75u128))),
        ],
        &[Coin::new(300u128, coin_info.denom.clone())],
    )
    .unwrap();

    let position = mock.query_position(&token_id);
    assert_eq!(position.coins.len(), 1);
    let asset_res = position.coins.first().unwrap();
    let expected_net_asset_amount = Uint128::from(299u128); // Deposit + Borrow - Repay - interest
    assert_eq!(asset_res.amount, expected_net_asset_amount);

    assert_eq!(position.debt_shares.len(), 0);

    let res = mock.query_total_debt_shares(&coin_info.denom);
    assert_eq!(res.shares, Uint128::zero());

    let coin = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(coin.amount, Uint128::from(299u128));

    let config = mock.query_config();
    let coin = mock.query_balance(&Addr::unchecked(config.red_bank), &coin_info.denom);
    assert_eq!(
        coin.amount,
        DEFAULT_RED_BANK_COIN_BALANCE.add(Uint128::from(1u128))
    );
}
