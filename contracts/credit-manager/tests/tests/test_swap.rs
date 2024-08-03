use cosmwasm_std::{coins, Addr, Coin, OverflowError, OverflowOperation::Sub, Uint128};
use mars_credit_manager::error::ContractError;
use mars_swapper_mock::contract::MOCK_SWAP_RESULT;
use mars_types::{
    credit_manager::{
        Action::{Deposit, SwapExactIn},
        ActionAmount, ActionCoin,
    },
    swapper::{OsmoRoute, OsmoSwap, SwapperRoute},
};

use super::helpers::{assert_err, uatom_info, uosmo_info, AccountToFund, MockEnv};

#[test]
fn only_token_owner_can_swap_for_account() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let another_user = Addr::unchecked("another_user");
    let res = mock.update_credit_account(
        &account_id,
        &another_user,
        vec![SwapExactIn {
            coin_in: ActionCoin {
                denom: "mars".to_string(),
                amount: ActionAmount::Exact(Uint128::new(12)),
            },
            denom_out: "osmo".to_string(),
            min_receive: Uint128::zero(),
            route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 101,
                    to: "osmo".to_string(),
                }],
            })),
        }],
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
fn denom_out_does_not_have_to_be_whitelisted() {
    let atom_info = uatom_info();
    let another_coin = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[atom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![Coin::new(10_000u128, atom_info.denom.clone())],
        })
        .build()
        .unwrap();

    let route = SwapperRoute::Osmo(OsmoRoute {
        swaps: vec![OsmoSwap {
            pool_id: 102,
            to: another_coin.denom.clone(),
        }],
    });
    let res =
        mock.query_swap_estimate(&atom_info.to_coin(10_000), &another_coin.denom, route.clone());
    assert_eq!(res.amount, MOCK_SWAP_RESULT);

    let account_id = mock.create_credit_account(&user).unwrap();
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom_info.to_coin(10_000)),
            SwapExactIn {
                coin_in: atom_info.to_action_coin(10_000),
                denom_out: another_coin.denom.clone(),
                min_receive: Uint128::zero(),
                route: Some(route),
            },
        ],
        &[atom_info.to_coin(10_000)],
    )
    .unwrap();

    // assert rover balance
    let atom_balance = mock.query_balance(&mock.rover, &atom_info.denom).amount;
    let osmo_balance = mock.query_balance(&mock.rover, &another_coin.denom).amount;
    assert_eq!(atom_balance, Uint128::zero());
    assert_eq!(osmo_balance, MOCK_SWAP_RESULT);

    // assert account position
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    let coin = position.deposits.first().unwrap();
    assert_eq!(coin.denom, another_coin.denom);
    assert_eq!(coin.amount, MOCK_SWAP_RESULT);
}

#[test]
fn no_amount_sent() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();

    let user = Addr::unchecked("user");
    let mut mock =
        MockEnv::new().set_params(&[osmo_info.clone(), atom_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![SwapExactIn {
            coin_in: osmo_info.to_action_coin(0),
            denom_out: atom_info.denom.clone(),
            min_receive: Uint128::zero(),
            route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 101,
                    to: atom_info.denom,
                }],
            })),
        }],
        &[],
    );

    assert_err(res, ContractError::NoAmount)
}

#[test]
fn user_has_zero_balance_for_swap_req() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();

    let user = Addr::unchecked("user");
    let mut mock =
        MockEnv::new().set_params(&[osmo_info.clone(), atom_info.clone()]).build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![SwapExactIn {
            coin_in: osmo_info.to_action_coin(10_000),
            denom_out: atom_info.denom.clone(),
            min_receive: Uint128::zero(),
            route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 101,
                    to: atom_info.denom,
                }],
            })),
        }],
        &[],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: Sub,
            operand1: "0".to_string(),
            operand2: "10000".to_string(),
        }),
    )
}

#[test]
fn min_receive_not_reached() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[osmo_info.clone(), atom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![Coin::new(10_000u128, osmo_info.denom.clone())],
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let swap_amt = Uint128::new(10_000);
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(osmo_info.to_coin(swap_amt.u128())),
            SwapExactIn {
                coin_in: osmo_info.to_action_coin(swap_amt.u128()),
                denom_out: atom_info.denom.clone(),
                min_receive: swap_amt + Uint128::one(), // min receive is 1 more than actual
                route: Some(SwapperRoute::Osmo(OsmoRoute {
                    swaps: vec![OsmoSwap {
                        pool_id: 101,
                        to: atom_info.denom.clone(),
                    }],
                })),
            },
        ],
        &[osmo_info.to_coin(swap_amt.u128())],
    )
    .unwrap_err();
}

#[test]
fn user_does_not_have_enough_balance_for_swap_req() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[osmo_info.clone(), atom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(300, osmo_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(osmo_info.to_coin(100)),
            SwapExactIn {
                coin_in: osmo_info.to_action_coin(10_000),
                denom_out: atom_info.denom.clone(),
                min_receive: Uint128::zero(),
                route: Some(SwapperRoute::Osmo(OsmoRoute {
                    swaps: vec![OsmoSwap {
                        pool_id: 101,
                        to: atom_info.denom,
                    }],
                })),
            },
        ],
        &[osmo_info.to_coin(100)],
    );

    assert_err(
        res,
        ContractError::Overflow(OverflowError {
            operation: Sub,
            operand1: "100".to_string(),
            operand2: "10000".to_string(),
        }),
    )
}

#[test]
fn swap_success_with_specified_amount() {
    let atom_info = uatom_info();
    let osmo_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[osmo_info.clone(), atom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![Coin::new(10_000u128, atom_info.denom.clone())],
        })
        .build()
        .unwrap();

    let route = SwapperRoute::Osmo(OsmoRoute {
        swaps: vec![OsmoSwap {
            pool_id: 101,
            to: osmo_info.denom.clone(),
        }],
    });
    let res = mock.query_swap_estimate(&atom_info.to_coin(10_000), &osmo_info.denom, route.clone());
    assert_eq!(res.amount, MOCK_SWAP_RESULT);

    let account_id = mock.create_credit_account(&user).unwrap();
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom_info.to_coin(10_000)),
            SwapExactIn {
                coin_in: atom_info.to_action_coin(10_000),
                denom_out: osmo_info.denom.clone(),
                min_receive: MOCK_SWAP_RESULT - Uint128::one(),
                route: Some(route),
            },
        ],
        &[atom_info.to_coin(10_000)],
    )
    .unwrap();

    // assert rover balance
    let atom_balance = mock.query_balance(&mock.rover, &atom_info.denom).amount;
    let osmo_balance = mock.query_balance(&mock.rover, &osmo_info.denom).amount;
    assert_eq!(atom_balance, Uint128::zero());
    assert_eq!(osmo_balance, MOCK_SWAP_RESULT);

    // assert account position
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.deposits.first().unwrap().denom, osmo_info.denom);
    assert_eq!(position.deposits.first().unwrap().amount, MOCK_SWAP_RESULT);
}

#[test]
fn swap_success_with_amount_none() {
    let atom_info = uatom_info();
    let osmo_info = uosmo_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[osmo_info.clone(), atom_info.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![Coin::new(10_000u128, atom_info.denom.clone())],
        })
        .build()
        .unwrap();

    let route = SwapperRoute::Osmo(OsmoRoute {
        swaps: vec![OsmoSwap {
            pool_id: 101,
            to: osmo_info.denom.clone(),
        }],
    });
    let res = mock.query_swap_estimate(&atom_info.to_coin(10_000), &osmo_info.denom, route.clone());
    assert_eq!(res.amount, MOCK_SWAP_RESULT);

    let account_id = mock.create_credit_account(&user).unwrap();
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom_info.to_coin(10_000)),
            SwapExactIn {
                coin_in: atom_info.to_action_coin_full_balance(),
                denom_out: osmo_info.denom.clone(),
                min_receive: MOCK_SWAP_RESULT - Uint128::one(),
                route: Some(route),
            },
        ],
        &[atom_info.to_coin(10_000)],
    )
    .unwrap();

    // assert rover balance
    let atom_balance = mock.query_balance(&mock.rover, &atom_info.denom).amount;
    let osmo_balance = mock.query_balance(&mock.rover, &osmo_info.denom).amount;
    assert_eq!(atom_balance, Uint128::zero());
    assert_eq!(osmo_balance, MOCK_SWAP_RESULT);

    // assert account position
    let position = mock.query_positions(&account_id);
    assert_eq!(position.deposits.len(), 1);
    assert_eq!(position.deposits.first().unwrap().denom, osmo_info.denom);
    assert_eq!(position.deposits.first().unwrap().amount, MOCK_SWAP_RESULT);
}
