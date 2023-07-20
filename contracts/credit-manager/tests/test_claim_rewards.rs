use cosmwasm_std::{Addr, Uint128};
use mars_rover::{error::ContractError, msg::execute::Action::ClaimRewards};

use crate::helpers::{assert_err, get_coin, uatom_info, ujake_info, uosmo_info, MockEnv};

pub mod helpers;

#[test]
fn claiming_rewards_when_having_none() {
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();

    let unclaimed = mock.query_unclaimed_rewards(&account_id);
    assert!(unclaimed.is_empty());

    let res = mock.update_credit_account(&account_id, &user, vec![ClaimRewards {}], &[]);
    assert_err(res, ContractError::NoAmount);
}

#[test]
fn claiming_a_single_reward() {
    let coin_info = uosmo_info();
    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();

    let unclaimed = mock.query_unclaimed_rewards(&account_id);
    assert!(unclaimed.is_empty());

    mock.add_incentive_reward(&account_id, coin_info.to_coin(123));

    let unclaimed = mock.query_unclaimed_rewards(&account_id);
    assert_eq!(unclaimed.len(), 1);

    mock.update_credit_account(&account_id, &user, vec![ClaimRewards {}], &[]).unwrap();

    // Check account id deposit balance
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 1);
    assert_eq!(positions.deposits.first().unwrap().amount, Uint128::new(123));
    assert_eq!(positions.deposits.first().unwrap().denom, coin_info.denom);

    // Ensure money is in bank module for credit manager
    let cm_balance = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(cm_balance.amount, Uint128::new(123));
}

#[test]
fn claiming_multiple_rewards() {
    let osmo_info = uosmo_info();
    let atom_info = uatom_info();
    let jake_info = ujake_info();

    let mut mock = MockEnv::new().build().unwrap();

    let user = Addr::unchecked("user");
    let account_id = mock.create_credit_account(&user).unwrap();

    let unclaimed = mock.query_unclaimed_rewards(&account_id);
    assert!(unclaimed.is_empty());

    mock.add_incentive_reward(&account_id, osmo_info.to_coin(123));
    mock.add_incentive_reward(&account_id, atom_info.to_coin(555));
    mock.add_incentive_reward(&account_id, jake_info.to_coin(12));

    let unclaimed = mock.query_unclaimed_rewards(&account_id);
    assert_eq!(unclaimed.len(), 3);

    mock.update_credit_account(&account_id, &user, vec![ClaimRewards {}], &[]).unwrap();

    // Check account id deposit balance
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 3);

    let osmo_claimed = get_coin(&osmo_info.denom, &positions.deposits);
    assert_eq!(osmo_claimed.amount, Uint128::new(123));

    let atom_claimed = get_coin(&atom_info.denom, &positions.deposits);
    assert_eq!(atom_claimed.amount, Uint128::new(555));

    let jake_claimed = get_coin(&jake_info.denom, &positions.deposits);
    assert_eq!(jake_claimed.amount, Uint128::new(12));

    // Ensure money is in bank module for credit manager
    let osmo_balance = mock.query_balance(&mock.rover, &osmo_info.denom);
    assert_eq!(osmo_balance.amount, Uint128::new(123));

    let atom_balance = mock.query_balance(&mock.rover, &atom_info.denom);
    assert_eq!(atom_balance.amount, Uint128::new(555));

    let jake_balance = mock.query_balance(&mock.rover, &jake_info.denom);
    assert_eq!(jake_balance.amount, Uint128::new(12));
}
