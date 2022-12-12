use cosmwasm_std::OverflowOperation::Sub;
use cosmwasm_std::{Addr, OverflowError, Uint128};

use mars_mock_zapper::contract::STARTING_LP_POOL_TOKENS;
use mars_rover::error::ContractError as RoverError;
use mars_rover::msg::execute::Action::{Deposit, ProvideLiquidity, WithdrawLiquidity};
use mars_rover::msg::instantiate::ConfigUpdates;

use crate::helpers::{
    assert_err, get_coin, lp_token_info, uatom_info, uosmo_info, AccountToFund, MockEnv,
};

pub mod helpers;

#[test]
fn test_only_token_owner_can_unzap_for_account() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let another_user = Addr::unchecked("another_user");
    let res = mock.update_credit_account(
        &account_id,
        &another_user,
        vec![WithdrawLiquidity {
            lp_token_denom: "xyz".to_string(),
            lp_token_amount: None,
        }],
        &[],
    );

    assert_err(
        res,
        RoverError::NotTokenOwner {
            user: another_user.into(),
            account_id,
        },
    )
}

#[test]
fn test_lp_token_in_must_be_whitelisted() {
    let lp_token = lp_token_info();
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![WithdrawLiquidity {
            lp_token_denom: lp_token.denom.clone(),
            lp_token_amount: Some(Uint128::new(100)),
        }],
        &[],
    );

    assert_err(res, RoverError::NotWhitelisted(lp_token.denom))
}

#[test]
fn test_coins_out_must_be_whitelisted() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), atom.clone(), osmo.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![atom.to_coin(300), osmo.to_coin(300)],
        })
        .build()
        .unwrap();

    // Seed zapper with denoms so test can estimate withdraws
    let account_id = mock.create_credit_account(&user).unwrap();
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom.to_coin(100)),
            Deposit(osmo.to_coin(50)),
            ProvideLiquidity {
                coins_in: vec![atom.to_coin(100), osmo.to_coin(50)],
                lp_token_out: lp_token.denom.clone(),
                minimum_receive: Uint128::zero(),
            },
        ],
        &[atom.to_coin(100), osmo.to_coin(50)],
    )
    .unwrap();

    // update config to disallow denoms out
    let config = mock.query_config();
    mock.update_config(
        &Addr::unchecked(config.admin.unwrap()),
        ConfigUpdates {
            allowed_coins: Some(vec![lp_token.denom.clone(), atom.denom]),
            ..Default::default()
        },
    )
    .unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![WithdrawLiquidity {
            lp_token_denom: lp_token.denom,
            lp_token_amount: Some(Uint128::new(100_000)),
        }],
        &[],
    );

    assert_err(res, RoverError::NotWhitelisted(osmo.denom))
}

#[test]
fn test_does_not_have_the_tokens_to_withdraw_liq() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), atom.clone(), osmo.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![atom.to_coin(300), osmo.to_coin(300)],
        })
        .build()
        .unwrap();

    // Seed zapper with denoms so test can estimate withdraws
    let account_id = mock.create_credit_account(&user).unwrap();
    let attempted_unzap_amount = 100_000_000_000;
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom.to_coin(100)),
            Deposit(osmo.to_coin(50)),
            ProvideLiquidity {
                coins_in: vec![atom.to_coin(100), osmo.to_coin(50)],
                lp_token_out: lp_token.denom.clone(),
                minimum_receive: Uint128::zero(),
            },
            WithdrawLiquidity {
                lp_token_denom: lp_token.denom,
                lp_token_amount: Some(Uint128::new(attempted_unzap_amount)),
            },
        ],
        &[atom.to_coin(100), osmo.to_coin(50)],
    );

    assert_err(
        res,
        RoverError::Overflow(OverflowError {
            operation: Sub,
            operand1: STARTING_LP_POOL_TOKENS.to_string(),
            operand2: attempted_unzap_amount.to_string(),
        }),
    )
}

#[test]
fn test_amount_zero_passed() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), atom.clone(), osmo.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![atom.to_coin(300), osmo.to_coin(300)],
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom.to_coin(100)),
            Deposit(osmo.to_coin(50)),
            ProvideLiquidity {
                coins_in: vec![atom.to_coin(100), osmo.to_coin(50)],
                lp_token_out: lp_token.denom.clone(),
                minimum_receive: Uint128::zero(),
            },
        ],
        &[atom.to_coin(100), osmo.to_coin(50)],
    )
    .unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![WithdrawLiquidity {
            lp_token_denom: lp_token.denom,
            lp_token_amount: Some(Uint128::zero()),
        }],
        &[],
    );

    assert_err(res, RoverError::NoAmount)
}

#[test]
fn test_amount_none_passed_with_no_balance() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), atom.clone(), osmo.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![atom.to_coin(300), osmo.to_coin(300)],
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![WithdrawLiquidity {
            lp_token_denom: lp_token.denom,
            lp_token_amount: None,
        }],
        &[],
    );

    assert_err(res, RoverError::NoAmount)
}

#[test]
fn test_successful_unzap_specified_amount() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), atom.clone(), osmo.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![atom.to_coin(300), osmo.to_coin(300)],
        })
        .build()
        .unwrap();

    // Seed zapper with denoms so test can estimate withdraws
    let account_id = mock.create_credit_account(&user).unwrap();
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom.to_coin(100)),
            Deposit(osmo.to_coin(50)),
            ProvideLiquidity {
                coins_in: vec![atom.to_coin(100), osmo.to_coin(50)],
                lp_token_out: lp_token.denom.clone(),
                minimum_receive: Uint128::zero(),
            },
            WithdrawLiquidity {
                lp_token_denom: lp_token.denom.clone(),
                lp_token_amount: Some(STARTING_LP_POOL_TOKENS),
            },
        ],
        &[atom.to_coin(100), osmo.to_coin(50)],
    )
    .unwrap();

    // Assert user's new position
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.coins.len(), 2);
    let atom_balance = get_coin(&atom.denom, &positions.coins);
    assert_eq!(atom_balance.amount, Uint128::new(100));
    let osmo_balance = get_coin(&osmo.denom, &positions.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(50));

    // assert rover actually has the tokens
    let lp_balance = mock.query_balance(&mock.rover, &lp_token.denom);
    assert_eq!(lp_balance.amount, Uint128::zero());
    let atom_balance = mock.query_balance(&mock.rover, &atom.denom);
    assert_eq!(atom_balance.amount, Uint128::new(100));
    let osmo_balance = mock.query_balance(&mock.rover, &osmo.denom);
    assert_eq!(osmo_balance.amount, Uint128::new(50));

    // assert coin balance of zapper contract
    let config = mock.query_config();
    let lp_balance = mock.query_balance(&Addr::unchecked(config.zapper.clone()), &lp_token.denom);
    assert_eq!(lp_balance.amount, Uint128::new(10_000_000)); // prefunded original amount
    let atom_balance = mock.query_balance(&Addr::unchecked(config.zapper.clone()), &atom.denom);
    assert_eq!(atom_balance.amount, Uint128::zero());
    let osmo_balance = mock.query_balance(&Addr::unchecked(config.zapper), &osmo.denom);
    assert_eq!(osmo_balance.amount, Uint128::zero());
}

#[test]
fn test_successful_unzap_unspecified_amount() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), atom.clone(), osmo.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![atom.to_coin(300), osmo.to_coin(300)],
        })
        .build()
        .unwrap();

    // Seed zapper with denoms so test can estimate withdraws
    let account_id = mock.create_credit_account(&user).unwrap();
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom.to_coin(100)),
            Deposit(osmo.to_coin(50)),
            ProvideLiquidity {
                coins_in: vec![atom.to_coin(100), osmo.to_coin(50)],
                lp_token_out: lp_token.denom.clone(),
                minimum_receive: Uint128::zero(),
            },
            WithdrawLiquidity {
                lp_token_denom: lp_token.denom.clone(),
                lp_token_amount: None,
            },
        ],
        &[atom.to_coin(100), osmo.to_coin(50)],
    )
    .unwrap();

    // Assert user's new position
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.coins.len(), 2);
    let atom_balance = get_coin(&atom.denom, &positions.coins);
    assert_eq!(atom_balance.amount, Uint128::new(100));
    let osmo_balance = get_coin(&osmo.denom, &positions.coins);
    assert_eq!(osmo_balance.amount, Uint128::new(50));

    // assert rover actually has the tokens
    let lp_balance = mock.query_balance(&mock.rover, &lp_token.denom);
    assert_eq!(lp_balance.amount, Uint128::zero());
    let atom_balance = mock.query_balance(&mock.rover, &atom.denom);
    assert_eq!(atom_balance.amount, Uint128::new(100));
    let osmo_balance = mock.query_balance(&mock.rover, &osmo.denom);
    assert_eq!(osmo_balance.amount, Uint128::new(50));

    // assert coin balance of zapper contract
    let config = mock.query_config();
    let lp_balance = mock.query_balance(&Addr::unchecked(config.zapper.clone()), &lp_token.denom);
    assert_eq!(lp_balance.amount, Uint128::new(10_000_000)); // prefunded original amount
    let atom_balance = mock.query_balance(&Addr::unchecked(config.zapper.clone()), &atom.denom);
    assert_eq!(atom_balance.amount, Uint128::zero());
    let osmo_balance = mock.query_balance(&Addr::unchecked(config.zapper), &osmo.denom);
    assert_eq!(osmo_balance.amount, Uint128::zero());
}
