use astroport_v5::{
    asset::{Asset, AssetInfo},
    incentives::InputSchedule,
};
use cosmwasm_std::{coin, Addr, Coin, Uint128};
use mars_testing::integration::mock_env::MockEnvBuilder;

use crate::helpers::default_asset_params;
mod helpers;

#[test]
fn can_be_first_staker() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    // Contracts
    let params = mock_env.params.clone();
    let incentives = mock_env.incentives.clone();
    let credit_manager = mock_env.credit_manager.clone();

    // Params
    let lp_denom = "factory12345";
    let lp_coin = coin(1_000_000_000, lp_denom.to_string());

    // Set asset params for lp token
    let (_, asset_params) = default_asset_params(lp_denom);
    params.init_params(&mut mock_env, asset_params);

    // Fund accounts
    let funded_amt = 10_000_000_000u128;
    mock_env.fund_account(&credit_manager, &[coin(funded_amt, lp_denom)]);

    incentives.stake_astro_lp(&mut mock_env, &credit_manager, "1".to_string(), lp_coin.clone());

    let astro_lp_balance =
        mock_env.query_balance(&mock_env.astro_incentives.contract_addr, lp_denom).unwrap();
    assert_eq!(astro_lp_balance, lp_coin)
}

// User A stakes lp in astro and claims rewards
#[test]
fn claim_rewards() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    // Contracts
    let params = mock_env.params.clone();
    let astro_incentives = mock_env.astro_incentives.clone();
    let incentives = mock_env.incentives.clone();
    let credit_manager = mock_env.credit_manager.clone();

    // Params
    let lp_denom = "factory12345";
    let reward_denom = "uusd";
    let lp_coin = Coin {
        denom: lp_denom.to_string(),
        amount: Uint128::new(1_000_000_000),
    };
    let reward_asset = Asset {
        info: AssetInfo::NativeToken {
            denom: reward_denom.to_string(),
        },
        amount: Uint128::new(10_000_000_000),
    };

    // Set asset params for lp token
    let (_, asset_params) = default_asset_params(lp_denom);
    params.init_params(&mut mock_env, asset_params);

    // Fund accounts
    let funded_amt = 10_000_000_000u128;
    mock_env.fund_account(&mock_env.owner.clone(), &[coin(funded_amt, reward_denom)]);
    mock_env.fund_account(&credit_manager, &[coin(funded_amt, lp_denom)]);

    // set up our astroport incentives
    let incentives_for_astro = &InputSchedule {
        reward: reward_asset.clone(),
        duration_periods: 1,
    };

    let rewards = vec![Coin::try_from(reward_asset.clone()).unwrap()];
    astro_incentives.set_incentive_schedule(&mut mock_env, lp_denom, incentives_for_astro, rewards);

    incentives.stake_astro_lp(&mut mock_env, &credit_manager, "1".to_string(), lp_coin);
    // increase blocks?
    mock_env.increment_by_blocks(1);

    let lp_rewards: Vec<Coin> =
        incentives.query_unclaimed_astroport_rewards(&mock_env, "1".to_string(), lp_denom).unwrap();

    assert_eq!(lp_rewards.len(), 1);
    let balance = mock_env.query_balance(&credit_manager, reward_denom).unwrap();
    assert_eq!(balance, coin(0, reward_denom));
    // claim rewards
    incentives
        .claim_astro_rewards(&mut mock_env, &credit_manager, "1".to_string(), lp_denom)
        .unwrap();

    // Ensure that balance of credit manager is updated with rewards paid
    let balance = mock_env.query_balance(&credit_manager, reward_denom).unwrap();
    assert_eq!(balance, lp_rewards[0]);
}

#[test]
fn claim_rewards_without_active_schedule() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    // Contracts
    let params = mock_env.params.clone();
    let incentives = mock_env.incentives.clone();
    let credit_manager = mock_env.credit_manager.clone();

    // Params
    let lp_denom = "factory12345";

    let lp_coin = Coin {
        denom: lp_denom.to_string(),
        amount: Uint128::new(1_000_000_000),
    };

    // Set asset params for lp token
    let (_, asset_params) = default_asset_params(lp_denom);
    params.init_params(&mut mock_env, asset_params);

    // Fund accounts
    let funded_amt = 1_000_000_000u128;
    mock_env.fund_account(&credit_manager, &[coin(funded_amt, lp_denom)]);

    incentives.stake_astro_lp(&mut mock_env, &credit_manager, "1".to_string(), lp_coin);

    mock_env.increment_by_blocks(1);

    // claim rewards
    incentives
        .claim_astro_rewards(&mut mock_env, &credit_manager, "1".to_string(), lp_denom)
        .unwrap();
    let lp_balance = mock_env.query_balance(&mock_env.credit_manager, lp_denom).unwrap();
    assert_eq!(lp_balance.amount, Uint128::new(0));
}

#[test]
fn unstake_claims_rewards() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    // Contracts
    let params = mock_env.params.clone();
    let astro_incentives = mock_env.astro_incentives.clone();
    let incentives = mock_env.incentives.clone();
    let credit_manager = mock_env.credit_manager.clone();

    let funded_amt = 2_000_000_000u128;

    // Params
    let lp_denom = "factory12345";
    let reward_denom = "uusd";

    let lp_coin = Coin {
        denom: lp_denom.to_string(),
        amount: Uint128::new(funded_amt),
    };

    let reward_asset = Asset {
        info: AssetInfo::NativeToken {
            denom: reward_denom.to_string(),
        },
        amount: Uint128::new(funded_amt),
    };

    // Set asset params for lp token
    let (_, asset_params) = default_asset_params(lp_denom);
    params.init_params(&mut mock_env, asset_params);

    // Fund accounts
    mock_env.fund_account(&mock_env.owner.clone(), &[coin(funded_amt, reward_denom)]);
    mock_env.fund_account(&credit_manager, &[coin(funded_amt, lp_denom)]);

    // set up our astroport incentives
    let incentives_for_astro = &InputSchedule {
        reward: reward_asset.clone(),
        duration_periods: 1,
    };

    let rewards = vec![Coin::try_from(reward_asset.clone()).unwrap()];
    astro_incentives.set_incentive_schedule(&mut mock_env, lp_denom, incentives_for_astro, rewards);

    incentives.stake_astro_lp(&mut mock_env, &credit_manager, "1".to_string(), lp_coin.clone());

    mock_env.increment_by_blocks(1);

    let lp_rewards: Vec<Coin> =
        incentives.query_unclaimed_astroport_rewards(&mock_env, "1".to_string(), lp_denom).unwrap();

    incentives.unstake_astro_lp(&mut mock_env, &credit_manager, "1".to_string(), lp_coin.clone());

    // Ensure that balance of credit manager is updated with rewards paid
    let reward_balance = mock_env.query_balance(&credit_manager, reward_denom).unwrap();
    assert_eq!(reward_balance, lp_rewards[0]);
    assert_eq!(lp_rewards.len(), 1);

    // Ensure our lp balance is incremented in credit manager
    let lp_balance = mock_env.query_balance(&credit_manager, lp_denom).unwrap();
    assert_eq!(lp_balance, lp_coin);
}
