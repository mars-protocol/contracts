use cosmwasm_std::{coins, Addr, Coin, Uint128};
use mars_testing::multitest::helpers::{uosmo_info, AccountToFund};
use mars_types::credit_manager::{Action, ActionAmount, ActionCoin};

use super::helpers::MockEnv;

#[test]
fn unstake_fails_if_no_lp_staked() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let account_id: String = mock.create_credit_account(&user).unwrap();
    let lp_denom = "factory12345";

    // Query staked LP, verify is 0
    let staked_lp = mock.query_staked_lp_position(&account_id, lp_denom);
    assert!(staked_lp.lp_coin.amount.is_zero());

    // Unstake
    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::UnstakeAstroLp {
            lp_token: ActionCoin {
                denom: lp_denom.to_string(),
                amount: ActionAmount::Exact(Uint128::new(100)),
            },
        }],
        &[],
    );
    assert!(res.is_err());
}

#[test]
fn unstake() {
    let user = Addr::unchecked("user");
    let lp_denom = "factory12345";

    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(100, lp_denom),
        })
        .build()
        .unwrap();

    let account_id: String = mock.create_credit_account(&user).unwrap();

    // Query staked LP, verify is 0
    let staked_lp_response = mock.query_staked_lp_position(&account_id, lp_denom);

    assert!(staked_lp_response.lp_coin.amount.is_zero());

    let lp_coin = Coin {
        denom: lp_denom.to_string(),
        amount: Uint128::new(100),
    };

    // stake
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(lp_coin.clone()),
            Action::StakeAstroLp {
                lp_token: ActionCoin::from(&lp_coin.clone()),
            },
        ],
        &[lp_coin],
    )
    .unwrap();

    // Unstake 50%
    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::UnstakeAstroLp {
            lp_token: ActionCoin {
                denom: lp_denom.to_string(),
                amount: ActionAmount::Exact(Uint128::new(50)),
            },
        }],
        &[],
    )
    .unwrap();

    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits[0].amount, Uint128::new(50));
    assert_eq!(positions.deposits[0].denom, lp_denom.to_string());

    // Entire remaining amount
    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::UnstakeAstroLp {
            lp_token: ActionCoin {
                denom: lp_denom.to_string(),
                amount: ActionAmount::AccountBalance,
            },
        }],
        &[],
    )
    .unwrap();

    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits[0].amount, Uint128::new(100));
    assert_eq!(positions.deposits[0].denom, lp_denom.to_string());
}

#[test]
fn unstake_claims_rewards() {
    let user = Addr::unchecked("user");
    let lp_denom = "factory12345";
    let coin_info = uosmo_info();

    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(100, lp_denom),
        })
        .build()
        .unwrap();

    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();
    let lp_amount = Uint128::new(100);
    let reward = Coin {
        denom: coin_info.denom.clone(),
        amount: Uint128::new(1000000),
    };

    let lp_coin = Coin {
        denom: lp_denom.to_string(),
        amount: lp_amount,
    };

    // stake
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(lp_coin.clone()),
            Action::StakeAstroLp {
                lp_token: ActionCoin::from(&lp_coin.clone()),
            },
        ],
        &[lp_coin],
    )
    .unwrap();

    // add rewards
    mock.add_astro_incentive_reward(&account_id, lp_denom, reward.clone());

    // Unstake 50%
    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::UnstakeAstroLp {
            lp_token: ActionCoin {
                denom: lp_denom.to_string(),
                amount: ActionAmount::Exact(Uint128::new(50)),
            },
        }],
        &[],
    )
    .unwrap();

    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 2);
    assert_eq!(positions.deposits[0].amount, Uint128::new(50));
    assert_eq!(positions.deposits[0].denom, lp_denom.to_string());
    assert_eq!(positions.deposits[1].amount, reward.amount);
    assert_eq!(positions.deposits[1].denom, coin_info.denom);
}
