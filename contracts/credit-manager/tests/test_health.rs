use std::ops::{Add, Mul};

use cosmwasm_std::{coins, Addr, Coin, Decimal, Uint128};
use mars_credit_manager::borrow::DEFAULT_DEBT_SHARES_PER_COIN_BORROWED;
use mars_math::{FractionMath, Fractional};
use mars_mock_oracle::msg::CoinPrice;
use mars_rover::{
    adapters::vault::VaultConfig,
    error::ContractError,
    msg::{
        execute::Action::{Borrow, Deposit, EnterVault},
        instantiate::{ConfigUpdates, VaultInstantiateConfig},
        query::DebtAmount,
    },
};

use crate::helpers::{
    assert_err, lp_token_info, uatom_info, ujake_info, unlocked_vault_info, uosmo_info,
    AccountToFund, CoinInfo, MockEnv,
};

pub mod helpers;

// Health scenarios:
// https://docs.google.com/spreadsheets/d/1YhydvetAkLywgyFjpzLIdRe-_z-KbjFOigiPuQBh-ac/edit#gid=1394903922

/// Action: User deposits 300 osmo (.25 price)
/// Health: assets_value: 75
///         debt value 0
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn only_assets_with_no_debts() {
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

    let deposit_amount = Uint128::new(300);
    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(coin_info.to_coin(deposit_amount.u128()))],
        &[Coin::new(deposit_amount.into(), coin_info.denom.clone())],
    )
    .unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 0);

    let health = mock.query_health(&account_id);
    let assets_value = deposit_amount.checked_mul_floor(coin_info.price).unwrap();
    assert_eq!(health.total_collateral_value, assets_value);
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.liquidation_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);
}

/// Step 1: User deposits 12 luna (100 price) and borrows 2 luna
/// Health: assets_value: 1400
///         debt value 200
///         liquidatable: false
///         above_max_ltv: false
/// Step 2: luna price goes to zero
/// Health: assets_value: 0
///         debt value 0 (still debt shares outstanding)
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn terra_ragnarok() {
    let coin_info = CoinInfo {
        denom: "uluna".to_string(),
        price: Decimal::from_atomics(100u128, 1).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
        liquidation_bonus: Decimal::from_atomics(15u128, 2).unwrap(),
    };

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

    let deposit_amount = Uint128::new(12);
    let borrow_amount = Uint128::new(2);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(coin_info.to_coin(deposit_amount.u128())),
            Borrow(coin_info.to_coin(borrow_amount.u128())),
        ],
        &[Coin::new(deposit_amount.into(), coin_info.denom.clone())],
    )
    .unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 1);

    let health = mock.query_health(&account_id);
    let assets_value = (deposit_amount + borrow_amount).checked_mul_floor(coin_info.price).unwrap();
    assert_eq!(health.total_collateral_value, assets_value);
    // Note: Simulated yield from mock_red_bank makes debt position more expensive
    let debts_value =
        borrow_amount.add(Uint128::new(1)).checked_mul_floor(coin_info.price).unwrap();
    assert_eq!(health.total_debt_value, debts_value);

    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_ratio(
            assets_value.checked_mul_floor(coin_info.liquidation_threshold).unwrap(),
            debts_value
        ))
    );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_ratio(
            assets_value.checked_mul_floor(coin_info.max_ltv).unwrap(),
            debts_value,
        ))
    );
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);

    mock.price_change(CoinPrice {
        denom: coin_info.denom,
        price: Decimal::zero(),
    });

    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 1);

    let health = mock.query_health(&account_id);
    assert_eq!(health.total_collateral_value, Uint128::zero());
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.liquidation_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);
}

/// Action: User borrows 100 osmo (at price of 1). Zero deposits.
/// Health: assets_value: 100
///         debt value: 100
///         liquidatable: true
///         above_max_ltv: true
#[test]
fn debts_no_assets() {
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

    let res =
        mock.update_credit_account(&account_id, &user, vec![Borrow(coin_info.to_coin(100))], &[]);

    assert_err(
        res,
        ContractError::AboveMaxLTV {
            account_id: account_id.clone(),
            max_ltv_health_factor: "0.68".to_string(),
        },
    );

    let position = mock.query_positions(&account_id);
    assert_eq!(position.account_id, account_id);
    assert_eq!(position.deposits.len(), 0);
    assert_eq!(position.debts.len(), 0);

    let health = mock.query_health(&account_id);
    assert_eq!(health.total_collateral_value, Uint128::zero());
    assert_eq!(health.total_debt_value, Uint128::zero());
    assert_eq!(health.liquidation_health_factor, None);
    assert_eq!(health.max_ltv_health_factor, None);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);
}

/// Step 1: User deposits 300 osmo and borrows 50 (at price of 2.3654)
/// Health: assets_value: 827.89
///         debt value: 121 (simulated interest incurred)
///         liquidatable: false
///         above_max_ltv: false
/// Step 2: User borrows 100
/// Health: assets_value: 1,064.43
///         debt value: 360 (simulated interest incurred)
///         liquidatable: false
///         above_max_ltv: false
/// Step 3: User borrows 100
///         AboveMaxLtv error thrown
#[test]
fn cannot_borrow_more_than_healthy() {
    let coin_info = ujake_info();

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

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(coin_info.to_coin(300)), Borrow(coin_info.to_coin(50))],
        &[Coin::new(Uint128::new(300).into(), coin_info.denom.clone())],
    )
    .unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.account_id, account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.debts.len(), 1);

    let health = mock.query_health(&account_id);
    let assets_value = Uint128::new(827);
    assert_eq!(health.total_collateral_value, assets_value);
    let debts_value = Uint128::new(120);
    assert_eq!(health.total_debt_value, debts_value);
    assert_eq!(health.liquidation_health_factor, Some(Decimal::from_ratio(454u128, 120u128)));
    assert_eq!(health.max_ltv_health_factor, Some(Decimal::from_ratio(413u128, 120u128)));
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);

    mock.update_credit_account(&account_id, &user, vec![Borrow(coin_info.to_coin(100))], &[])
        .unwrap();

    let res =
        mock.update_credit_account(&account_id, &user, vec![Borrow(coin_info.to_coin(150))], &[]);

    assert_err(
        res,
        ContractError::AboveMaxLTV {
            account_id: account_id.clone(),
            max_ltv_health_factor: "0.990223463687150837".to_string(),
        },
    );

    // All valid on step 2 as well (meaning step 3 did not go through)
    let health = mock.query_health(&account_id);
    let assets_value = Uint128::new(1064);
    assert_eq!(health.total_collateral_value, assets_value);
    let debts_value = Uint128::new(359);
    assert_eq!(health.total_debt_value, debts_value);
    assert_eq!(health.liquidation_health_factor, Some(Decimal::from_ratio(585u128, 359u128)));
    assert_eq!(health.max_ltv_health_factor, Some(Decimal::from_ratio(532u128, 359u128)));
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);
}

/// Step 1: User deposits 300 osmo (2.3654) and borrows 50 atom (price 10.2)
/// Health: liquidatable: false
///         above_max_ltv: false
/// Step 2: Atom's price increases to 24
/// Health: liquidatable: false
///         above_max_ltv: true
/// Step 3: User borrows 2 atom
///         AboveMaxLtv error thrown
/// Step 4: Atom's price increases to 35
/// Health: liquidatable: true
///         above_max_ltv: true
#[test]
fn cannot_borrow_more_but_not_liquidatable() {
    let uosmo_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(23654u128, 4).unwrap(),
        max_ltv: Decimal::from_atomics(5u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(55u128, 2).unwrap(),
        liquidation_bonus: Decimal::from_atomics(2u128, 1).unwrap(),
    };
    let uatom_info = CoinInfo {
        denom: "uatom".to_string(),
        price: Decimal::from_atomics(102u128, 1).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(75u128, 2).unwrap(),
        liquidation_bonus: Decimal::from_atomics(2u128, 1).unwrap(),
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(uosmo_info.to_coin(300)), Borrow(uatom_info.to_coin(50))],
        &[Coin::new(300, uosmo_info.denom)],
    )
    .unwrap();

    let health = mock.query_health(&account_id);
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);

    mock.price_change(CoinPrice {
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(24u128, 0).unwrap(),
    });

    let health = mock.query_health(&account_id);
    assert!(!health.liquidatable);
    assert!(health.above_max_ltv);

    let res =
        mock.update_credit_account(&account_id, &user, vec![Borrow(uatom_info.to_coin(2))], &[]);

    assert_err(
        res,
        ContractError::AboveMaxLTV {
            account_id: account_id.clone(),
            max_ltv_health_factor: "0.946759259259259259".to_string(),
        },
    );

    mock.price_change(CoinPrice {
        denom: uatom_info.denom,
        price: Decimal::from_atomics(35u128, 0).unwrap(),
    });

    let health = mock.query_health(&account_id);
    assert!(health.liquidatable);
    assert!(health.above_max_ltv);
}

/// Actions: User deposits 300 osmo (5265478965.412365487125 price)
///          and borrows 49 atom ( price)
/// Health: assets_value: 1569456334491.12991516325
///         debt value 350615100.25
///         liquidatable: false
///         above_max_ltv: false
#[test]
fn assets_and_ltv_lqdt_adjusted_value() {
    let uosmo_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(5265478965412365487125u128, 12).unwrap(),
        max_ltv: Decimal::from_atomics(6u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_bonus: Decimal::from_atomics(15u128, 2).unwrap(),
    };
    let uatom_info = CoinInfo {
        denom: "uatom".to_string(),
        price: Decimal::from_atomics(7012302005u128, 3).unwrap(),
        max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(9u128, 1).unwrap(),
        liquidation_bonus: Decimal::from_atomics(12u128, 2).unwrap(),
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let deposit_amount = Uint128::new(298);
    let borrowed_amount = Uint128::new(49);
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(uosmo_info.to_coin(deposit_amount.u128())),
            Borrow(uatom_info.to_coin(borrowed_amount.u128())),
        ],
        &[Coin::new(deposit_amount.into(), uosmo_info.denom.clone())],
    )
    .unwrap();

    let position = mock.query_positions(&account_id);
    assert_eq!(position.account_id, account_id);
    assert_eq!(position.deposits.len(), 2);
    assert_eq!(position.debts.len(), 1);

    let health = mock.query_health(&account_id);
    assert_eq!(
        health.total_collateral_value,
        deposit_amount
            .checked_mul_floor(uosmo_info.price)
            .unwrap()
            .add(borrowed_amount.checked_mul_floor(uatom_info.price).unwrap())
    );
    assert_eq!(
        health.total_debt_value,
        Uint128::new(350_615_100) // with simulated interest
    );
    let lqdt_adjusted_assets_value = deposit_amount
        .checked_mul_floor(uosmo_info.price)
        .unwrap()
        .checked_mul_floor(uosmo_info.liquidation_threshold)
        .unwrap()
        .add(
            borrowed_amount
                .checked_mul_floor(uatom_info.price)
                .unwrap()
                .checked_mul_floor(uatom_info.liquidation_threshold)
                .unwrap(),
        );
    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_ratio(
            lqdt_adjusted_assets_value,
            (borrowed_amount + Uint128::one()).checked_mul_floor(uatom_info.price).unwrap()
        ))
    );
    let ltv_adjusted_assets_value = deposit_amount
        .checked_mul_floor(uosmo_info.price)
        .unwrap()
        .checked_mul_floor(uosmo_info.max_ltv)
        .unwrap()
        .add(
            borrowed_amount
                .checked_mul_floor(uatom_info.price)
                .unwrap()
                .checked_mul_floor(uatom_info.max_ltv)
                .unwrap(),
        );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_ratio(
            ltv_adjusted_assets_value,
            (borrowed_amount + Uint128::one()).checked_mul_floor(uatom_info.price).unwrap()
        ))
    );
    assert!(!health.liquidatable);
    assert!(!health.above_max_ltv);
}

/// User A: Borrows 30 osmo
///         Borrows 49 atom
///         Deposits 298 osmo
/// User B: Borrows 24 atom
///         Deposits 101 osmo
/// Test validates User A's debt value & health factors
#[test]
fn debt_value() {
    let uosmo_info = CoinInfo {
        denom: "uosmo".to_string(),
        price: Decimal::from_atomics(5265478965412365487125u128, 12).unwrap(),
        max_ltv: Decimal::from_atomics(3u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(5u128, 1).unwrap(),
        liquidation_bonus: Decimal::from_atomics(2u128, 1).unwrap(),
    };
    let uatom_info = CoinInfo {
        denom: "uatom".to_string(),
        price: Decimal::from_atomics(7012302005u128, 3).unwrap(),
        max_ltv: Decimal::from_atomics(8u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(9u128, 1).unwrap(),
        liquidation_bonus: Decimal::from_atomics(1u128, 1).unwrap(),
    };

    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: user_a.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: user_b.clone(),
            funds: coins(140, uosmo_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id_a = mock.create_credit_account(&user_a).unwrap();
    let account_id_b = mock.create_credit_account(&user_b).unwrap();

    let user_a_deposit_amount_osmo = Uint128::new(298);
    let user_a_borrowed_amount_atom = Uint128::new(49);
    let user_a_borrowed_amount_osmo = Uint128::new(30);

    mock.update_credit_account(
        &account_id_a,
        &user_a,
        vec![
            Borrow(uatom_info.to_coin(user_a_borrowed_amount_atom.u128())),
            Borrow(uosmo_info.to_coin(user_a_borrowed_amount_osmo.u128())),
            Deposit(uosmo_info.to_coin(user_a_deposit_amount_osmo.u128())),
        ],
        &[Coin::new(user_a_deposit_amount_osmo.into(), uosmo_info.denom.clone())],
    )
    .unwrap();

    let interim_red_bank_debt = mock.query_red_bank_debt(&uatom_info.denom);

    let user_b_deposit_amount = Uint128::new(101);
    let user_b_borrowed_amount_atom = Uint128::new(24);

    mock.update_credit_account(
        &account_id_b,
        &user_b,
        vec![
            Borrow(uatom_info.to_coin(user_b_borrowed_amount_atom.u128())),
            Deposit(uosmo_info.to_coin(user_b_deposit_amount.u128())),
        ],
        &[Coin::new(user_b_deposit_amount.into(), uosmo_info.denom.clone())],
    )
    .unwrap();

    let position_a = mock.query_positions(&account_id_a);
    assert_eq!(position_a.account_id, account_id_a);
    assert_eq!(position_a.deposits.len(), 2);
    assert_eq!(position_a.debts.len(), 2);

    let health = mock.query_health(&account_id_a);
    assert!(!health.above_max_ltv);
    assert!(!health.liquidatable);

    let red_bank_atom_debt = mock.query_red_bank_debt(&uatom_info.denom);

    let user_a_debt_shares_atom =
        user_a_borrowed_amount_atom.mul(DEFAULT_DEBT_SHARES_PER_COIN_BORROWED);
    assert_eq!(user_a_debt_shares_atom, find_by_denom(&uatom_info.denom, &position_a.debts).shares);

    let position_b = mock.query_positions(&account_id_b);
    let user_b_debt_shares_atom = user_a_debt_shares_atom
        .multiply_ratio(user_b_borrowed_amount_atom, interim_red_bank_debt.amount);
    assert_eq!(user_b_debt_shares_atom, find_by_denom(&uatom_info.denom, &position_b.debts).shares);

    let red_bank_atom_res = mock.query_total_debt_shares(&uatom_info.denom);

    assert_eq!(red_bank_atom_res.shares, user_a_debt_shares_atom + user_b_debt_shares_atom);

    let user_a_owed_atom = red_bank_atom_debt
        .amount
        .checked_mul_ceil(Fractional(user_a_debt_shares_atom, red_bank_atom_res.shares))
        .unwrap();
    let user_a_owed_atom_value = user_a_owed_atom.checked_mul_floor(uatom_info.price).unwrap();

    let osmo_debt_value =
        (user_a_borrowed_amount_osmo + Uint128::one()).checked_mul_floor(uosmo_info.price).unwrap();

    let total_debt_value = user_a_owed_atom_value.add(osmo_debt_value);
    assert_eq!(health.total_debt_value, total_debt_value);

    let lqdt_adjusted_assets_value = user_a_deposit_amount_osmo
        .add(user_a_borrowed_amount_osmo)
        .checked_mul_floor(uosmo_info.price)
        .unwrap()
        .checked_mul_floor(uosmo_info.liquidation_threshold)
        .unwrap()
        .add(
            user_a_borrowed_amount_atom
                .checked_mul_floor(uatom_info.price)
                .unwrap()
                .checked_mul_floor(uatom_info.liquidation_threshold)
                .unwrap(),
        );

    assert_eq!(
        health.liquidation_health_factor,
        Some(Decimal::from_ratio(lqdt_adjusted_assets_value, total_debt_value))
    );

    let ltv_adjusted_assets_value = user_a_deposit_amount_osmo
        .add(user_a_borrowed_amount_osmo)
        .checked_mul_floor(uosmo_info.price)
        .unwrap()
        .checked_mul_floor(uosmo_info.max_ltv)
        .unwrap()
        .add(
            user_a_borrowed_amount_atom
                .checked_mul_floor(uatom_info.price)
                .unwrap()
                .checked_mul_floor(uatom_info.max_ltv)
                .unwrap(),
        );
    assert_eq!(
        health.max_ltv_health_factor,
        Some(Decimal::from_ratio(ltv_adjusted_assets_value, total_debt_value))
    );
}

#[test]
fn delisted_deposits_drop_max_ltv() {
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, uosmo_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Deposit(uosmo_info.to_coin(300)), Borrow(uatom_info.to_coin(100))],
        &[uosmo_info.to_coin(300)],
    )
    .unwrap();

    let prev_health = mock.query_health(&account_id);

    // Remove uosmo from the coin whitelist
    let res = mock.query_config();
    mock.update_config(
        &Addr::unchecked(res.owner.unwrap()),
        ConfigUpdates {
            allowed_coins: Some(vec![uatom_info.denom]),
            ..Default::default()
        },
    )
    .unwrap();

    let curr_health = mock.query_health(&account_id);

    // Values should be the same
    assert_eq!(prev_health.total_debt_value, curr_health.total_debt_value);
    assert_eq!(prev_health.total_collateral_value, curr_health.total_collateral_value);

    assert_eq!(prev_health.liquidation_health_factor, curr_health.liquidation_health_factor);
    assert_eq!(
        prev_health.liquidation_threshold_adjusted_collateral,
        curr_health.liquidation_threshold_adjusted_collateral
    );
    assert_eq!(prev_health.liquidatable, curr_health.liquidatable);

    // Should have been changed due to de-listing
    assert_ne!(prev_health.above_max_ltv, curr_health.above_max_ltv);
    assert_ne!(prev_health.max_ltv_adjusted_collateral, curr_health.max_ltv_adjusted_collateral);
    assert_ne!(prev_health.max_ltv_health_factor, curr_health.max_ltv_health_factor);
    assert_eq!(curr_health.max_ltv_health_factor, Some(Decimal::raw(811881188118811881u128)));
}

#[test]
fn delisted_vaults_drop_max_ltv() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();
    let atom = uatom_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), atom.clone()])
        .vault_configs(&[leverage_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![lp_token.to_coin(300)],
        })
        .build()
        .unwrap();

    let vault = mock.get_vault(&leverage_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(200)),
            Borrow(atom.to_coin(100)),
            EnterVault {
                vault,
                coin: lp_token.to_action_coin(200),
            },
        ],
        &[lp_token.to_coin(200)],
    )
    .unwrap();

    let prev_health = mock.query_health(&account_id);

    let vault_configs = mock.query_vault_configs(None, None);
    let v = vault_configs.first().unwrap();
    let new_vault_config = VaultInstantiateConfig {
        vault: v.vault.clone(),
        config: VaultConfig {
            deposit_cap: v.config.deposit_cap.clone(),
            max_ltv: v.config.max_ltv,
            liquidation_threshold: v.config.liquidation_threshold,
            whitelisted: false,
        },
    };

    // Remove uosmo from the coin whitelist
    let res = mock.query_config();
    mock.update_config(
        &Addr::unchecked(res.owner.unwrap()),
        ConfigUpdates {
            vault_configs: Some(vec![new_vault_config]),
            ..Default::default()
        },
    )
    .unwrap();

    let curr_health = mock.query_health(&account_id);

    // // Values should be the same
    assert_eq!(prev_health.total_debt_value, curr_health.total_debt_value);
    assert_eq!(prev_health.total_collateral_value, curr_health.total_collateral_value);

    assert_eq!(prev_health.liquidation_health_factor, curr_health.liquidation_health_factor);
    assert_eq!(
        prev_health.liquidation_threshold_adjusted_collateral,
        curr_health.liquidation_threshold_adjusted_collateral
    );
    assert_eq!(prev_health.liquidatable, curr_health.liquidatable);

    // Should have been changed due to de-listing
    assert_ne!(prev_health.above_max_ltv, curr_health.above_max_ltv);
    assert_ne!(prev_health.max_ltv_adjusted_collateral, curr_health.max_ltv_adjusted_collateral);
    assert_ne!(prev_health.max_ltv_health_factor, curr_health.max_ltv_health_factor);
    assert_eq!(curr_health.max_ltv_health_factor, Some(Decimal::raw(811881188118811881u128)));
}

fn find_by_denom<'a>(denom: &'a str, shares: &'a [DebtAmount]) -> &'a DebtAmount {
    shares.iter().find(|item| item.denom == *denom).unwrap()
}
