use cosmwasm_std::{Addr, Decimal, OverflowError, OverflowOperation::Sub, Uint128};
use mars_rover::{
    error::ContractError as RoverError,
    msg::execute::{
        Action::{Deposit, ProvideLiquidity, WithdrawLiquidity},
        ActionAmount, ActionCoin,
    },
};
use mars_v2_zapper_mock::contract::STARTING_LP_POOL_TOKENS;

use crate::helpers::{
    assert_err, get_coin, lp_token_info, uatom_info, uosmo_info, AccountToFund, MockEnv,
};

pub mod helpers;

#[test]
fn only_token_owner_can_unzap_for_account() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let another_user = Addr::unchecked("another_user");
    let res = mock.update_credit_account(
        &account_id,
        &another_user,
        vec![WithdrawLiquidity {
            lp_token: ActionCoin {
                denom: "xyz".to_string(),
                amount: ActionAmount::AccountBalance,
            },
            slippage: Decimal::zero(),
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
fn does_not_have_the_tokens_to_withdraw_liq() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), atom.clone(), osmo.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![atom.to_coin(300), osmo.to_coin(300)],
        })
        .build()
        .unwrap();

    // Seed zapper with denoms so test can estimate withdraws
    let account_id = mock.create_credit_account(&user).unwrap();
    let attempted_unzap_amount = 100_000_000_000u128;
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom.to_coin(100)),
            Deposit(osmo.to_coin(50)),
            ProvideLiquidity {
                coins_in: vec![atom.to_action_coin(100), osmo.to_action_coin(50)],
                lp_token_out: lp_token.denom.clone(),
                slippage: Decimal::zero(),
            },
            WithdrawLiquidity {
                lp_token: lp_token.to_action_coin(attempted_unzap_amount),
                slippage: Decimal::zero(),
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
fn amount_zero_passed() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), atom.clone(), osmo.clone()])
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
                coins_in: vec![atom.to_action_coin(100), osmo.to_action_coin(50)],
                lp_token_out: lp_token.denom.clone(),
                slippage: Decimal::zero(),
            },
        ],
        &[atom.to_coin(100), osmo.to_coin(50)],
    )
    .unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![WithdrawLiquidity {
            lp_token: lp_token.to_action_coin(0),
            slippage: Decimal::zero(),
        }],
        &[],
    );

    assert_err(res, RoverError::NoAmount)
}

#[test]
fn amount_none_passed_with_no_balance() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), atom.clone(), osmo.clone()])
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
            lp_token: lp_token.to_action_coin_full_balance(),
            slippage: Decimal::zero(),
        }],
        &[],
    );

    assert_err(res, RoverError::NoAmount)
}

#[test]
fn slippage_too_high() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let max_slippage = Decimal::percent(20);
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), atom.clone(), osmo.clone()])
        .max_slippage(max_slippage)
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![atom.to_coin(300), osmo.to_coin(300)],
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    let slippage = max_slippage + Decimal::one();
    let err = mock
        .update_credit_account(
            &account_id,
            &user,
            vec![
                Deposit(atom.to_coin(100)),
                Deposit(osmo.to_coin(50)),
                ProvideLiquidity {
                    coins_in: vec![atom.to_action_coin(100), osmo.to_action_coin(50)],
                    lp_token_out: lp_token.denom.clone(),
                    slippage: Decimal::zero(),
                },
                WithdrawLiquidity {
                    lp_token: lp_token.to_action_coin(STARTING_LP_POOL_TOKENS.u128()),
                    slippage,
                },
            ],
            &[atom.to_coin(100), osmo.to_coin(50)],
        )
        .unwrap_err();

    let contract_err: mars_rover::error::ContractError = err.downcast().unwrap();
    assert_eq!(
        contract_err,
        mars_rover::error::ContractError::SlippageExceeded {
            slippage,
            max_slippage
        }
    );
}

#[test]
fn successful_unzap_specified_amount() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), atom.clone(), osmo.clone()])
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
                coins_in: vec![atom.to_action_coin(100), osmo.to_action_coin(50)],
                lp_token_out: lp_token.denom.clone(),
                slippage: Decimal::zero(),
            },
            WithdrawLiquidity {
                lp_token: lp_token.to_action_coin(STARTING_LP_POOL_TOKENS.u128()),
                slippage: Decimal::percent(10),
            },
        ],
        &[atom.to_coin(100), osmo.to_coin(50)],
    )
    .unwrap();

    // Assert user's new position
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 2);
    let atom_balance = get_coin(&atom.denom, &positions.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(100));
    let osmo_balance = get_coin(&osmo.denom, &positions.deposits);
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
fn successful_unzap_unspecified_amount() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), atom.clone(), osmo.clone()])
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
                coins_in: vec![atom.to_action_coin(100), osmo.to_action_coin(50)],
                lp_token_out: lp_token.denom.clone(),
                slippage: Decimal::zero(),
            },
            WithdrawLiquidity {
                lp_token: lp_token.to_action_coin_full_balance(),
                slippage: Decimal::zero(),
            },
        ],
        &[atom.to_coin(100), osmo.to_coin(50)],
    )
    .unwrap();

    // Assert user's new position
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 2);
    let atom_balance = get_coin(&atom.denom, &positions.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(100));
    let osmo_balance = get_coin(&osmo.denom, &positions.deposits);
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
