use astroport_v5::{
    asset::{Asset, AssetInfo},
    incentives::InputSchedule,
};
use cosmwasm_std::{coin, Addr, Coin, Uint128};
use mars_testing::{
    assert_eq_vec,
    integration::mock_env::{MockEnv, MockEnvBuilder},
};

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

#[test]
fn unstake_all_positions_resets_state_correctly() {
    // Params
    let lp_denom = "factory/neturon1234/astroport/share";
    let stake_lp_amount = 1000000;

    let lp_coin = Coin {
        denom: lp_denom.to_string(),
        amount: Uint128::new(stake_lp_amount),
    };

    let helper = AstroIncentivesTestHelper::new(None, None, None, Some(lp_denom.to_string()));

    let mut mock_env = helper.mock;

    // Contracts
    let incentives = mock_env.incentives.clone();
    let astro_incentives = mock_env.astro_incentives.clone();

    astro_incentives.set_incentive_schedule(
        &mut mock_env,
        lp_denom,
        &helper.incentive_schedule,
        helper.rewards.clone(),
    );

    let credit_manager = mock_env.credit_manager.clone();

    incentives.stake_astro_lp(&mut mock_env, &credit_manager, "1".to_string(), lp_coin.clone());
    incentives.stake_astro_lp(&mut mock_env, &credit_manager, "2".to_string(), lp_coin.clone());

    mock_env.increment_by_blocks(1);

    incentives.unstake_astro_lp(&mut mock_env, &credit_manager, "1".to_string(), lp_coin.clone());
    mock_env.increment_by_blocks(1);
    incentives.unstake_astro_lp(&mut mock_env, &credit_manager, "2".to_string(), lp_coin.clone());
    mock_env.increment_by_blocks(1);

    incentives.stake_astro_lp(&mut mock_env, &credit_manager, "1".to_string(), lp_coin.clone());

    // verify incentives are still 0 - we have not progressed time (blocks) so we should not have rewards
    let rewards =
        incentives.query_unclaimed_astroport_rewards(&mock_env, "1".to_string(), lp_denom).unwrap();
    assert_eq_vec(vec![], rewards);
}

/// Test helpers to build state for astro incentives

struct AstroIncentivesTestHelper {
    rewards: Vec<Coin>,
    incentive_schedule: InputSchedule,
    mock: MockEnv,
}

impl AstroIncentivesTestHelper {
    fn new(
        owner: Option<String>,
        funded_assets: Option<Vec<Coin>>,
        incentive_schedule: Option<InputSchedule>,
        lp_denom: Option<String>,
    ) -> Self {
        let owner = owner.unwrap_or("owner".to_string());
        let mut mock_env = MockEnvBuilder::new(None, Addr::unchecked(owner)).build();

        let funded_amt = 10_000_000_000u128;
        let default_reward_denom = "factory/neutron1234/rewards";
        let lp_denom = lp_denom.unwrap_or("factory/neturon1234/astroport/share".to_string());

        // Rewards
        let default_reward_asset = Asset {
            info: AssetInfo::NativeToken {
                denom: default_reward_denom.to_string(),
            },
            amount: Uint128::new(funded_amt),
        };

        let default_incentives_schedule = &InputSchedule {
            reward: default_reward_asset.clone(),
            duration_periods: 1u64,
        };

        let incentive_schedule = incentive_schedule.unwrap_or(default_incentives_schedule.clone());

        let rewards = vec![Coin::try_from(incentive_schedule.reward.clone()).unwrap()];

        // Funded assets - required to fund our mocks
        let mut default_funded_assets = rewards.clone();
        default_funded_assets.push(Coin {
            denom: lp_denom.clone(),
            amount: funded_amt.into(),
        });
        let funded_assets = funded_assets.unwrap_or(default_funded_assets);

        let credit_manager = mock_env.credit_manager.clone();
        let params = mock_env.params.clone();

        // Set asset params for lp token
        let (_, asset_params) = default_asset_params(lp_denom.as_str());
        params.init_params(&mut mock_env, asset_params);

        // Fund accounts we need funded
        mock_env.fund_account(&credit_manager, &funded_assets);
        mock_env.fund_account(&mock_env.owner.clone(), &funded_assets);

        AstroIncentivesTestHelper {
            rewards,
            incentive_schedule,
            mock: mock_env,
        }
    }
}
