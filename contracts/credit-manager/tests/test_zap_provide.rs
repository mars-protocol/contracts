use std::ops::Mul;

use cosmwasm_std::{Addr, Decimal, OverflowError, OverflowOperation::Sub, Uint128};
use mars_rover::{
    error::ContractError as RoverError,
    msg::execute::{
        Action::{Deposit, ProvideLiquidity, WithdrawLiquidity},
        ActionAmount, ActionCoin,
    },
};
use mars_v2_zapper_mock::{contract::STARTING_LP_POOL_TOKENS, error::ContractError};

use crate::helpers::{
    assert_err, get_coin, lp_token_info, uatom_info, ujake_info, uosmo_info, AccountToFund, MockEnv,
};

pub mod helpers;

#[test]
fn only_token_owner_can_zap_for_account() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new().build().unwrap();
    let account_id = mock.create_credit_account(&user).unwrap();

    let another_user = Addr::unchecked("another_user");
    let res = mock.update_credit_account(
        &account_id,
        &another_user,
        vec![ProvideLiquidity {
            coins_in: vec![],
            lp_token_out: "".to_string(),
            slippage: Decimal::percent(5),
        }],
        &[],
    );

    assert_err(
        res,
        RoverError::NotTokenOwner {
            user: another_user.clone().into(),
            account_id: account_id.clone(),
        },
    );
}

#[test]
fn does_not_have_enough_tokens_to_provide_liq() {
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
        vec![
            Deposit(atom.to_coin(100)),
            Deposit(osmo.to_coin(50)),
            ProvideLiquidity {
                coins_in: vec![atom.to_action_coin(100), osmo.to_action_coin(200)],
                lp_token_out: lp_token.denom,
                slippage: Decimal::zero(),
            },
        ],
        &[atom.to_coin(100), osmo.to_coin(50)],
    );

    assert_err(
        res,
        RoverError::Overflow(OverflowError {
            operation: Sub,
            operand1: "50".to_string(),
            operand2: "200".to_string(),
        }),
    )
}

#[test]
fn lp_token_out_must_be_whitelisted() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[atom.clone(), osmo.clone()])
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
        vec![
            Deposit(atom.to_coin(100)),
            Deposit(osmo.to_coin(50)),
            ProvideLiquidity {
                coins_in: vec![atom.to_action_coin(100), osmo.to_action_coin(200)],
                lp_token_out: lp_token.denom.clone(),
                slippage: Decimal::zero(),
            },
        ],
        &[atom.to_coin(100), osmo.to_coin(50)],
    );

    assert_err(res, RoverError::NotWhitelisted(lp_token.denom))
}

#[test]
fn coins_in_must_be_whitelisted() {
    let atom = uatom_info();
    let osmo = uosmo_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), osmo.clone()])
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
        vec![ProvideLiquidity {
            coins_in: vec![atom.to_action_coin(100), osmo.to_action_coin(200)],
            lp_token_out: lp_token.denom,
            slippage: Decimal::zero(),
        }],
        &[],
    );

    assert_err(res, RoverError::NotWhitelisted(atom.denom))
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
                    lp_token_out: lp_token.denom,
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
fn wrong_denom_provided() {
    let atom = uatom_info();
    let jake = ujake_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), atom.clone(), jake.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![atom.to_coin(300), jake.to_coin(300)],
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();
    let err = mock
        .update_credit_account(
            &account_id,
            &user,
            vec![
                Deposit(atom.to_coin(100)),
                Deposit(jake.to_coin(50)),
                ProvideLiquidity {
                    coins_in: vec![atom.to_action_coin(100), jake.to_action_coin(50)],
                    lp_token_out: lp_token.denom,
                    slippage: Decimal::zero(),
                },
            ],
            &[atom.to_coin(100), jake.to_coin(50)],
        )
        .unwrap_err();

    let contract_err: ContractError = err.downcast().unwrap();
    assert_eq!(
        contract_err,
        ContractError::RequirementsNotMet("ujake is unexpected for lp_token_out_denom".to_string())
    );
}

#[test]
fn successful_zap() {
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
    let estimate =
        mock.estimate_provide_liquidity(&lp_token.denom, &[atom.to_coin(100), osmo.to_coin(50)]);
    let slippage = Decimal::percent(5);
    let slippage_adjusted = estimate * (Decimal::one() - slippage);
    assert_eq!(slippage_adjusted, Uint128::new(950_000)); // 1_000_000 * .95

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom.to_coin(100)),
            Deposit(osmo.to_coin(50)),
            ProvideLiquidity {
                coins_in: vec![atom.to_action_coin(100), osmo.to_action_coin(50)],
                lp_token_out: lp_token.denom.clone(),
                slippage,
            },
        ],
        &[atom.to_coin(100), osmo.to_coin(50)],
    )
    .unwrap();

    // assert follow up estimate can be made (calculates ratio from first deposit)
    let estimate =
        mock.estimate_provide_liquidity(&lp_token.denom, &[atom.to_coin(300), osmo.to_coin(150)]);
    assert_eq!(estimate, Uint128::new(3008928)); // ~3x the size as first deposit

    // assert user's new position
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 1);
    let lp_balance = get_coin(&lp_token.denom, &positions.deposits);
    assert_eq!(lp_balance.amount, STARTING_LP_POOL_TOKENS);

    // assert rover actually has the tokens
    let lp_balance = mock.query_balance(&mock.rover, &lp_token.denom);
    assert_eq!(lp_balance.amount, STARTING_LP_POOL_TOKENS);
    let atom_balance = mock.query_balance(&mock.rover, &atom.denom);
    assert_eq!(atom_balance.amount, Uint128::zero());
    let osmo_balance = mock.query_balance(&mock.rover, &osmo.denom);
    assert_eq!(osmo_balance.amount, Uint128::zero());

    // assert coin balance of zapper contract
    let config = mock.query_config();
    let lp_balance = mock.query_balance(&Addr::unchecked(config.zapper.clone()), &lp_token.denom);
    // prefunded minus minted
    assert_eq!(lp_balance.amount, Uint128::new(10_000_000) - STARTING_LP_POOL_TOKENS);
    let atom_balance = mock.query_balance(&Addr::unchecked(config.zapper.clone()), &atom.denom);
    assert_eq!(atom_balance.amount, Uint128::new(100));
    let osmo_balance = mock.query_balance(&Addr::unchecked(config.zapper), &osmo.denom);
    assert_eq!(osmo_balance.amount, Uint128::new(50));
}

#[test]
fn can_provide_unbalanced() {
    let atom = uatom_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[lp_token.clone(), atom.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![atom.to_coin(300)],
        })
        .build()
        .unwrap();

    let account_id = mock.create_credit_account(&user).unwrap();

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom.to_coin(100)),
            ProvideLiquidity {
                coins_in: vec![atom.to_action_coin(100)],
                lp_token_out: lp_token.denom.clone(),
                slippage: Decimal::percent(5),
            },
        ],
        &[atom.to_coin(100)],
    )
    .unwrap();

    let config = mock.query_config();

    // assert user's new position
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 1);
    let lp_balance = get_coin(&lp_token.denom, &positions.deposits);
    assert_eq!(lp_balance.amount, STARTING_LP_POOL_TOKENS);

    // assert coin balance of zapper contract
    let atom_balance = mock.query_balance(&Addr::unchecked(config.zapper.clone()), &atom.denom);
    assert_eq!(atom_balance.amount, Uint128::new(100));

    mock.update_credit_account(
        &account_id,
        &user,
        vec![WithdrawLiquidity {
            lp_token: ActionCoin {
                denom: lp_token.denom.clone(),
                amount: ActionAmount::Exact(STARTING_LP_POOL_TOKENS.multiply_ratio(1u128, 2u128)),
            },
            slippage: Decimal::zero(),
        }],
        &[],
    )
    .unwrap();

    // assert user's new position (withdrew half)
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 2);
    let lp_balance = get_coin(&lp_token.denom, &positions.deposits);
    assert_eq!(lp_balance.amount, STARTING_LP_POOL_TOKENS.multiply_ratio(1u128, 2u128));
    let atom_balance = get_coin(&atom.denom, &positions.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(50));

    // assert coin balance of zapper contract
    let atom_balance = mock.query_balance(&Addr::unchecked(config.zapper), &atom.denom);
    assert_eq!(atom_balance.amount, Uint128::new(50));
}

#[test]
fn order_does_not_matter() {
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
    let estimate =
        mock.estimate_provide_liquidity(&lp_token.denom, &[atom.to_coin(100), osmo.to_coin(50)]);
    let slippage = Decimal::percent(5);
    let slippage_adjusted = estimate * (Decimal::one() - slippage);
    assert_eq!(slippage_adjusted, Uint128::new(950_000)); // 1_000_000 * .95

    // order A
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom.to_coin(100)),
            Deposit(osmo.to_coin(50)),
            ProvideLiquidity {
                coins_in: vec![atom.to_action_coin(100), osmo.to_action_coin(50)],
                lp_token_out: lp_token.denom.clone(),
                slippage,
            },
        ],
        &[atom.to_coin(100), osmo.to_coin(50)],
    )
    .unwrap();

    // order B
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(atom.to_coin(100)),
            Deposit(osmo.to_coin(50)),
            ProvideLiquidity {
                coins_in: vec![osmo.to_action_coin(50), atom.to_action_coin(100)],
                lp_token_out: lp_token.denom.clone(),
                slippage,
            },
        ],
        &[atom.to_coin(100), osmo.to_coin(50)],
    )
    .unwrap();

    // assert user's new position
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 1);
    let lp_balance = get_coin(&lp_token.denom, &positions.deposits);
    assert_eq!(lp_balance.amount, STARTING_LP_POOL_TOKENS.mul(Uint128::new(2)));
}
