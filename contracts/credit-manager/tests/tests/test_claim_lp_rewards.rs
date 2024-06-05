use cosmwasm_std::{Addr, Coin, Uint128};
use mars_credit_manager::error::ContractError;
use mars_testing::multitest::helpers::{assert_err, coin_info};
use mars_types::credit_manager::Action;

use super::helpers::{uosmo_info, MockEnv};

#[test]
fn claiming_rewards_when_there_are_none() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();
    let lp_denom = "factory12345";

    let unclaimed = mock.query_staked_astro_lp_rewards(&account_id, lp_denom);
    assert!(unclaimed.is_empty());

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::ClaimAstroLpRewards {
            lp_denom: lp_denom.to_string(),
        }],
        &[],
    );
    assert_err(res, ContractError::NoAmount);
}

#[test]
fn claiming_a_single_reward() {
    let coin_info = uosmo_info();
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();
    let lp_denom = "factory12345";
    let reward = Coin {
        denom: coin_info.denom.clone(),
        amount: Uint128::new(1000000),
    };

    let unclaimed = mock.query_staked_astro_lp_rewards(&account_id, lp_denom);
    assert!(unclaimed.is_empty());

    mock.add_astro_incentive_reward(&account_id, lp_denom, reward.clone());

    let unclaimed = mock.query_staked_astro_lp_rewards(&account_id, lp_denom);
    assert_eq!(unclaimed.len(), 1);

    // Check account id deposit balance
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 0);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::ClaimAstroLpRewards {
            lp_denom: lp_denom.to_string(),
        }],
        &[],
    )
    .unwrap();

    // Check account id deposit balance
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 1);
    assert_eq!(positions.deposits[0].denom, reward.denom.clone());
    assert_eq!(positions.deposits[0].amount, reward.amount);

    let coin = mock.query_balance(&mock.rover, &reward.denom);
    assert_eq!(coin.amount, reward.amount);
}

#[test]
fn claiming_multiple_rewards() {
    let osmo_info = uosmo_info();
    let atom_info = coin_info("atom");
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();
    let lp_denom = "factory12345";

    let reward_1 = Coin {
        denom: osmo_info.denom.clone(),
        amount: Uint128::new(1000000),
    };

    let reward_2 = Coin {
        denom: atom_info.denom.clone(),
        amount: Uint128::new(1000000),
    };

    let unclaimed = mock.query_staked_astro_lp_rewards(&account_id, lp_denom);
    assert!(unclaimed.is_empty());

    mock.add_astro_incentive_reward(&account_id, lp_denom, reward_1.clone());
    mock.add_astro_incentive_reward(&account_id, lp_denom, reward_2.clone());

    let unclaimed = mock.query_staked_astro_lp_rewards(&account_id, lp_denom);
    assert_eq!(unclaimed.len(), 2);

    // Check account id deposit balance
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 0);

    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::ClaimAstroLpRewards {
            lp_denom: lp_denom.to_string(),
        }],
        &[],
    )
    .unwrap();

    // Check account id deposit balance
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 2);
    assert_eq!(positions.deposits[1].denom, reward_1.denom.clone());
    assert_eq!(positions.deposits[1].amount, reward_1.amount);
    assert_eq!(positions.deposits[0].denom, reward_2.denom.clone());
    assert_eq!(positions.deposits[0].amount, reward_2.amount);

    // Check contract has assets
    let reward_balance_1 = mock.query_balance(&mock.rover, &reward_1.denom);
    assert_eq!(reward_balance_1.amount, reward_1.amount);
    let reward_balance_2 = mock.query_balance(&mock.rover, &reward_2.denom);
    assert_eq!(reward_balance_2.amount, reward_2.amount);

    // Assert no LP coins in the contract
    let lp_coin = mock.query_balance(&mock.rover, lp_denom);
    assert!(lp_coin.amount.is_zero());
}
