use cosmwasm_std::{coin, coins, Addr, Coin, Uint128};
use mars_credit_manager::error::ContractError;
use mars_testing::multitest::helpers::{assert_err, uosmo_info, AccountToFund};
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
    assert_err(
        res,
        ContractError::InsufficientFunds {
            requested: Uint128::new(100u128),
            available: Uint128::zero(),
        },
    );
}

#[test]
fn unstake() {
    let user = Addr::unchecked("user");
    let lp_denom = "factory12345";

    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(200, lp_denom),
        })
        .build()
        .unwrap();

    let account_id: String = mock.create_credit_account(&user).unwrap();

    // Query staked LP, verify is 0
    let staked_lp_response = mock.query_staked_lp_position(&account_id, lp_denom);

    assert!(staked_lp_response.lp_coin.amount.is_zero());

    let lp_coin = Coin {
        denom: lp_denom.to_string(),
        amount: Uint128::new(200),
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
        &[lp_coin.clone()],
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
    assert_eq!(positions.staked_astro_lps.len(), 1);
    assert_eq!(positions.staked_astro_lps[0].denom, lp_denom.to_string());
    assert_eq!(positions.staked_astro_lps[0].amount, Uint128::new(150));
    assert_eq!(positions.deposits[0].denom, lp_denom.to_string());
    assert_eq!(positions.deposits[0].amount, Uint128::new(50));

    // Assert correct lp balance in contract
    let cm_lp_coin = mock.query_balance(&mock.rover, lp_denom);
    assert_eq!(positions.deposits[0].denom, cm_lp_coin.denom);
    assert_eq!(positions.deposits[0].amount, cm_lp_coin.amount);

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
    assert_eq!(positions.staked_astro_lps.len(), 0);
    assert_eq!(positions.deposits.len(), 1);
    assert_eq!(positions.deposits[0].denom, lp_denom.to_string());
    assert_eq!(positions.deposits[0].amount, Uint128::new(200));

    // Assert correct lp balance in contract
    let cm_lp_coin = mock.query_balance(&mock.rover, lp_denom);
    assert_eq!(cm_lp_coin.amount, lp_coin.amount);
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

    let reward = coin(1000000u128, coin_info.denom.clone());
    let lp_coin = coin(lp_amount.u128(), lp_denom.to_string());

    // stake
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(lp_coin.clone()),
            Action::StakeAstroLp {
                lp_token: ActionCoin {
                    denom: lp_denom.to_string(),
                    amount: ActionAmount::AccountBalance,
                },
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
    assert_eq!(positions.staked_astro_lps.len(), 1);
    assert_eq!(positions.staked_astro_lps[0].denom, lp_denom.to_string());
    assert_eq!(positions.staked_astro_lps[0].amount, Uint128::new(50));
    assert_eq!(positions.deposits.len(), 2);
    assert_eq!(positions.deposits[0].denom, lp_denom.to_string());
    assert_eq!(positions.deposits[0].amount, Uint128::new(50));
    assert_eq!(positions.deposits[1].denom, coin_info.denom);
    assert_eq!(positions.deposits[1].amount, reward.amount);

    // Assert correct lp balance in contract
    let lp_coin = mock.query_balance(&mock.rover, lp_denom);
    assert_eq!(positions.deposits[0].amount, lp_coin.amount);
}
