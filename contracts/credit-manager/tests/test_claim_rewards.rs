use cosmwasm_std::{Addr, Uint128};
use mars_params::{msg::AssetParamsUpdate::AddOrUpdate, types::hls::HlsAssetType};
use mars_rover::{
    error::ContractError,
    msg::execute::Action::{Borrow, ClaimRewards, Deposit},
};

use crate::helpers::{
    assert_err, get_coin, lp_token_info, uatom_info, ujake_info, uosmo_info, AccountToFund, MockEnv,
};

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
    assert_eq!(positions.deposits.len(), 0);

    // Ensure money is in user's wallet
    let balance = mock.query_balance(&mock.rover, &coin_info.denom);
    assert_eq!(balance.amount, Uint128::zero());

    let balance = mock.query_balance(&user, &coin_info.denom);
    assert_eq!(balance.amount, Uint128::new(123));
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
    assert_eq!(positions.deposits.len(), 0);

    // Ensure money is in user's wallet
    let osmo_balance = mock.query_balance(&mock.rover, &osmo_info.denom);
    assert_eq!(osmo_balance.amount, Uint128::zero());

    let atom_balance = mock.query_balance(&mock.rover, &atom_info.denom);
    assert_eq!(atom_balance.amount, Uint128::zero());

    let jake_balance = mock.query_balance(&mock.rover, &jake_info.denom);
    assert_eq!(jake_balance.amount, Uint128::zero());

    let osmo_balance = mock.query_balance(&user, &osmo_info.denom);
    assert_eq!(osmo_balance.amount, Uint128::new(123));

    let atom_balance = mock.query_balance(&user, &atom_info.denom);
    assert_eq!(atom_balance.amount, Uint128::new(555));

    let jake_balance = mock.query_balance(&user, &jake_info.denom);
    assert_eq!(jake_balance.amount, Uint128::new(12));
}

#[test]
fn claiming_by_hls_account() {
    let atom_info = uatom_info();
    let osmo_info = uosmo_info();
    let jake_info = ujake_info();
    let lp_token = lp_token_info();

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .set_params(&[atom_info.clone(), osmo_info.clone(), jake_info.clone(), lp_token.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![lp_token.to_coin(300)],
        })
        .build()
        .unwrap();

    // Add assets to correlations of Atom in params contract
    let mut asset_params = mock.query_asset_params(&atom_info.denom);
    let hls = asset_params.credit_manager.hls.as_mut().unwrap();
    hls.correlations.push(HlsAssetType::Coin {
        denom: jake_info.denom.clone(),
    });
    hls.correlations.push(HlsAssetType::Coin {
        denom: lp_token.denom.clone(),
    });
    mock.update_asset_params(AddOrUpdate {
        params: asset_params.into(),
    });

    let account_id = mock.create_hls_account(&user);

    let lp_deposit_amount = 300;
    let atom_borrow_amount = 150;

    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(lp_deposit_amount)),
            Borrow(atom_info.to_coin(atom_borrow_amount)),
        ],
        &[lp_token.to_coin(lp_deposit_amount)],
    )
    .unwrap();

    let unclaimed = mock.query_unclaimed_rewards(&account_id);
    assert!(unclaimed.is_empty());

    mock.add_incentive_reward(&account_id, osmo_info.to_coin(123));
    mock.add_incentive_reward(&account_id, jake_info.to_coin(12));

    let unclaimed = mock.query_unclaimed_rewards(&account_id);
    assert_eq!(unclaimed.len(), 2);

    mock.update_credit_account(&account_id, &user, vec![ClaimRewards {}], &[]).unwrap();

    // Check account id deposit balance
    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.deposits.len(), 2);
    let lp_token_coin = get_coin(&lp_token.denom, &positions.deposits);
    assert_eq!(lp_token_coin.amount, Uint128::new(lp_deposit_amount));
    let atom_coin = get_coin(&atom_info.denom, &positions.deposits);
    assert_eq!(atom_coin.amount, Uint128::new(atom_borrow_amount));

    // Ensure money is in user's wallet
    let osmo_balance = mock.query_balance(&mock.rover, &osmo_info.denom);
    assert_eq!(osmo_balance.amount, Uint128::zero());
    let jake_balance = mock.query_balance(&mock.rover, &jake_info.denom);
    assert_eq!(jake_balance.amount, Uint128::zero());
    let osmo_balance = mock.query_balance(&user, &osmo_info.denom);
    assert_eq!(osmo_balance.amount, Uint128::new(123));
    let jake_balance = mock.query_balance(&user, &jake_info.denom);
    assert_eq!(jake_balance.amount, Uint128::new(12));
}
