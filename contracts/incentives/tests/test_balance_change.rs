use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{attr, Addr, Decimal, Response, Timestamp, Uint128};

use mars_outpost::incentives::msg::ExecuteMsg;
use mars_outpost::incentives::AssetIncentive;
use mars_testing::MockEnvParams;

use mars_incentives::contract::{execute, execute_balance_change, query_user_unclaimed_rewards};
use mars_incentives::state::{ASSET_INCENTIVES, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS};

use crate::helpers::setup_test;
use mars_incentives::helpers::{asset_incentive_compute_index, user_compute_accrued_rewards};

mod helpers;

#[test]
fn test_execute_balance_change_noops() {
    let mut deps = setup_test();

    // non existing incentive returns a no op
    let info = mock_info("ma_asset", &[]);
    let msg = ExecuteMsg::BalanceChange {
        user_address: Addr::unchecked("user"),
        user_balance_before: Uint128::new(100000),
        total_supply_before: Uint128::new(100000),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(res, Response::default())
}

#[test]
fn test_balance_change_zero_emission() {
    let mut deps = setup_test();
    let ma_asset_address = Addr::unchecked("ma_asset");
    let user_address = Addr::unchecked("user");
    let asset_incentive_index = Decimal::from_ratio(1_u128, 2_u128);

    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            &ma_asset_address,
            &AssetIncentive {
                emission_per_second: Uint128::zero(),
                index: asset_incentive_index,
                last_updated: 500_000,
            },
        )
        .unwrap();

    let info = mock_info("ma_asset", &[]);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(600_000),
        ..Default::default()
    });
    let msg = ExecuteMsg::BalanceChange {
        user_address: Addr::unchecked("user"),
        user_balance_before: Uint128::new(100_000),
        total_supply_before: Uint128::new(100_000),
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let expected_accrued_rewards =
        user_compute_accrued_rewards(Uint128::new(100_000), Decimal::zero(), asset_incentive_index)
            .unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/incentives/balance_change"),
            attr("ma_asset", "ma_asset"),
            attr("user", "user"),
            attr("rewards_accrued", expected_accrued_rewards),
            attr("asset_index", asset_incentive_index.to_string()),
        ]
    );

    // asset incentive index stays the same
    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, &ma_asset_address).unwrap();
    assert_eq!(asset_incentive.index, asset_incentive_index);
    assert_eq!(asset_incentive.last_updated, 600_000);

    // user index is set to asset's index
    let user_asset_index =
        USER_ASSET_INDICES.load(deps.as_ref().storage, (&user_address, &ma_asset_address)).unwrap();
    assert_eq!(user_asset_index, asset_incentive_index);

    // rewards get updated
    let user_unclaimed_rewards =
        USER_UNCLAIMED_REWARDS.load(deps.as_ref().storage, &user_address).unwrap();
    assert_eq!(user_unclaimed_rewards, expected_accrued_rewards)
}

#[test]
fn test_balance_change_user_with_zero_balance() {
    let mut deps = setup_test();
    let ma_asset_address = Addr::unchecked("ma_asset");
    let user_address = Addr::unchecked("user");

    let start_index = Decimal::from_ratio(1_u128, 2_u128);
    let emission_per_second = Uint128::new(100);
    let total_supply = Uint128::new(100_000);
    let time_last_updated = 500_000_u64;
    let time_contract_call = 600_000_u64;

    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            &ma_asset_address,
            &AssetIncentive {
                emission_per_second,
                index: start_index,
                last_updated: time_last_updated,
            },
        )
        .unwrap();

    let info = mock_info("ma_asset", &[]);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(time_contract_call),
        ..Default::default()
    });
    let msg = ExecuteMsg::BalanceChange {
        user_address: user_address.clone(),
        user_balance_before: Uint128::zero(),
        total_supply_before: total_supply,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let expected_index = asset_incentive_compute_index(
        start_index,
        emission_per_second,
        total_supply,
        time_last_updated,
        time_contract_call,
    )
    .unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/incentives/balance_change"),
            attr("ma_asset", "ma_asset"),
            attr("user", "user"),
            attr("rewards_accrued", "0"),
            attr("asset_index", expected_index.to_string()),
        ]
    );

    // asset incentive gets updated
    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, &ma_asset_address).unwrap();
    assert_eq!(asset_incentive.index, expected_index);
    assert_eq!(asset_incentive.last_updated, time_contract_call);

    // user index is set to asset's index
    let user_asset_index =
        USER_ASSET_INDICES.load(deps.as_ref().storage, (&user_address, &ma_asset_address)).unwrap();
    assert_eq!(user_asset_index, expected_index);

    // no new rewards
    let user_unclaimed_rewards =
        USER_UNCLAIMED_REWARDS.may_load(deps.as_ref().storage, &user_address).unwrap();
    assert_eq!(user_unclaimed_rewards, None)
}

#[test]
fn test_with_zero_previous_balance_and_asset_with_zero_index_accumulates_rewards() {
    let mut deps = setup_test();
    let ma_asset_address = Addr::unchecked("ma_asset");
    let user_address = Addr::unchecked("user");

    let start_index = Decimal::zero();
    let emission_per_second = Uint128::new(100);
    let time_last_updated = 500_000_u64;
    let time_contract_call = 600_000_u64;

    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            &ma_asset_address,
            &AssetIncentive {
                emission_per_second,
                index: start_index,
                last_updated: time_last_updated,
            },
        )
        .unwrap();

    {
        let info = mock_info("ma_asset", &[]);
        let env = mars_testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call),
            ..Default::default()
        });
        let msg = ExecuteMsg::BalanceChange {
            user_address: user_address.clone(),
            user_balance_before: Uint128::zero(),
            total_supply_before: Uint128::zero(),
        };
        // Execute balance changed, this is the first mint of the asset, so previous total
        // supply and user balance is 0
        execute(deps.as_mut(), env, info, msg).unwrap();
    }

    {
        // Some time passes and we query the user rewards, expected value should not be 0
        let user_balance = Uint128::new(100_000);
        let total_supply = Uint128::new(100_000);
        deps.querier.set_cw20_total_supply(ma_asset_address.clone(), total_supply);
        deps.querier.set_cw20_balances(ma_asset_address, &[(user_address, user_balance)]);
        let env = mars_testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call + 1000),
            ..Default::default()
        });
        let rewards_query =
            query_user_unclaimed_rewards(deps.as_ref(), env, String::from("user")).unwrap();
        assert_eq!(Uint128::new(1000).checked_mul(emission_per_second).unwrap(), rewards_query);
    }
}

#[test]
fn test_set_new_asset_incentive_user_non_zero_balance() {
    let mut deps = setup_test();
    let user_address = Addr::unchecked("user");

    // set cw20 balance for user
    let ma_asset_address = Addr::unchecked("ma_asset");
    let total_supply = Uint128::new(100_000);
    let user_balance = Uint128::new(10_000);

    deps.querier.set_cw20_total_supply(ma_asset_address.clone(), total_supply);
    deps.querier
        .set_cw20_balances(ma_asset_address.clone(), &[(user_address.clone(), user_balance)]);

    // set asset incentive
    {
        let time_last_updated = 500_000_u64;
        let emission_per_second = Uint128::new(100);
        let asset_incentive_index = Decimal::zero();

        ASSET_INCENTIVES
            .save(
                deps.as_mut().storage,
                &ma_asset_address,
                &AssetIncentive {
                    emission_per_second,
                    index: asset_incentive_index,
                    last_updated: time_last_updated,
                },
            )
            .unwrap();
    }

    // first query
    {
        let time_contract_call = 600_000_u64;

        let env = mars_testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call),
            ..Default::default()
        });

        let unclaimed_rewards =
            query_user_unclaimed_rewards(deps.as_ref(), env, "user".to_string()).unwrap();
        // 100_000 s * 100 MARS/s * 1/10th cw20 supply
        let expected_unclaimed_rewards = Uint128::new(1_000_000);
        assert_eq!(unclaimed_rewards, expected_unclaimed_rewards);
    }

    // increase user ma_asset balance
    {
        let time_contract_call = 700_000_u64;
        let user_balance = Uint128::new(25_000);

        deps.querier
            .set_cw20_balances(ma_asset_address.clone(), &[(user_address.clone(), user_balance)]);

        let env = mars_testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call),
            ..Default::default()
        });

        let info = mock_info(ma_asset_address.as_ref(), &[]);

        execute_balance_change(
            deps.as_mut(),
            env,
            info,
            user_address,
            Uint128::new(10_000),
            total_supply,
        )
        .unwrap();
    }

    // second query
    {
        let time_contract_call = 800_000_u64;

        let env = mars_testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call),
            ..Default::default()
        });

        let unclaimed_rewards =
            query_user_unclaimed_rewards(deps.as_ref(), env, "user".to_string()).unwrap();
        let expected_unclaimed_rewards = Uint128::new(
            // 200_000 s * 100 MARS/s * 1/10th cw20 supply +
            2_000_000 +
                // 100_000 s * 100 MARS/s * 1/4 cw20 supply
                2_500_000,
        );
        assert_eq!(unclaimed_rewards, expected_unclaimed_rewards);
    }
}

#[test]
fn test_balance_change_user_non_zero_balance() {
    let mut deps = setup_test();
    let ma_asset_address = Addr::unchecked("ma_asset");
    let user_address = Addr::unchecked("user");

    let emission_per_second = Uint128::new(100);
    let total_supply = Uint128::new(100_000);

    let mut expected_asset_incentive_index = Decimal::from_ratio(1_u128, 2_u128);
    let mut expected_time_last_updated = 500_000_u64;
    let mut expected_accumulated_rewards = Uint128::zero();

    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            &ma_asset_address,
            &AssetIncentive {
                emission_per_second,
                index: expected_asset_incentive_index,
                last_updated: expected_time_last_updated,
            },
        )
        .unwrap();

    let info = mock_info("ma_asset", &[]);

    // first call no previous rewards
    {
        let time_contract_call = 600_000_u64;
        let user_balance = Uint128::new(10_000);

        let env = mars_testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call),
            ..Default::default()
        });
        let msg = ExecuteMsg::BalanceChange {
            user_address: user_address.clone(),
            user_balance_before: user_balance,
            total_supply_before: total_supply,
        };
        let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

        expected_asset_incentive_index = asset_incentive_compute_index(
            expected_asset_incentive_index,
            emission_per_second,
            total_supply,
            expected_time_last_updated,
            time_contract_call,
        )
        .unwrap();

        let expected_accrued_rewards = user_compute_accrued_rewards(
            user_balance,
            Decimal::zero(),
            expected_asset_incentive_index,
        )
        .unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "outposts/incentives/balance_change"),
                attr("ma_asset", "ma_asset"),
                attr("user", "user"),
                attr("rewards_accrued", expected_accrued_rewards),
                attr("asset_index", expected_asset_incentive_index.to_string()),
            ]
        );

        // asset incentive gets updated
        expected_time_last_updated = time_contract_call;

        let asset_incentive =
            ASSET_INCENTIVES.load(deps.as_ref().storage, &ma_asset_address).unwrap();
        assert_eq!(asset_incentive.index, expected_asset_incentive_index);
        assert_eq!(asset_incentive.last_updated, expected_time_last_updated);

        // user index is set to asset's index
        let user_asset_index = USER_ASSET_INDICES
            .load(deps.as_ref().storage, (&user_address, &ma_asset_address))
            .unwrap();
        assert_eq!(user_asset_index, expected_asset_incentive_index);

        // user gets new rewards
        let user_unclaimed_rewards =
            USER_UNCLAIMED_REWARDS.load(deps.as_ref().storage, &user_address).unwrap();
        expected_accumulated_rewards += expected_accrued_rewards;
        assert_eq!(user_unclaimed_rewards, expected_accumulated_rewards)
    }

    // Second call accumulates new rewards
    {
        let time_contract_call = 700_000_u64;
        let user_balance = Uint128::new(20_000);

        let env = mars_testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call),
            ..Default::default()
        });
        let msg = ExecuteMsg::BalanceChange {
            user_address: user_address.clone(),
            user_balance_before: user_balance,
            total_supply_before: total_supply,
        };
        let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

        let previous_user_index = expected_asset_incentive_index;
        expected_asset_incentive_index = asset_incentive_compute_index(
            expected_asset_incentive_index,
            emission_per_second,
            total_supply,
            expected_time_last_updated,
            time_contract_call,
        )
        .unwrap();

        let expected_accrued_rewards = user_compute_accrued_rewards(
            user_balance,
            previous_user_index,
            expected_asset_incentive_index,
        )
        .unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "outposts/incentives/balance_change"),
                attr("ma_asset", "ma_asset"),
                attr("user", "user"),
                attr("rewards_accrued", expected_accrued_rewards),
                attr("asset_index", expected_asset_incentive_index.to_string()),
            ]
        );

        // asset incentive gets updated
        expected_time_last_updated = time_contract_call;

        let asset_incentive =
            ASSET_INCENTIVES.load(deps.as_ref().storage, &ma_asset_address).unwrap();
        assert_eq!(asset_incentive.index, expected_asset_incentive_index);
        assert_eq!(asset_incentive.last_updated, expected_time_last_updated);

        // user index is set to asset's index
        let user_asset_index = USER_ASSET_INDICES
            .load(deps.as_ref().storage, (&user_address, &ma_asset_address))
            .unwrap();
        assert_eq!(user_asset_index, expected_asset_incentive_index);

        // user gets new rewards
        let user_unclaimed_rewards =
            USER_UNCLAIMED_REWARDS.load(deps.as_ref().storage, &user_address).unwrap();
        expected_accumulated_rewards += expected_accrued_rewards;
        assert_eq!(user_unclaimed_rewards, expected_accumulated_rewards)
    }

    // Third call same block does not change anything
    {
        let time_contract_call = 700_000_u64;
        let user_balance = Uint128::new(20_000);

        let env = mars_testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call),
            ..Default::default()
        });
        let msg = ExecuteMsg::BalanceChange {
            user_address: user_address.clone(),
            user_balance_before: user_balance,
            total_supply_before: total_supply,
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "outposts/incentives/balance_change"),
                attr("ma_asset", "ma_asset"),
                attr("user", "user"),
                attr("rewards_accrued", "0"),
                attr("asset_index", expected_asset_incentive_index.to_string()),
            ]
        );

        // asset incentive is still the same
        let asset_incentive =
            ASSET_INCENTIVES.load(deps.as_ref().storage, &ma_asset_address).unwrap();
        assert_eq!(asset_incentive.index, expected_asset_incentive_index);
        assert_eq!(asset_incentive.last_updated, expected_time_last_updated);

        // user index is still the same
        let user_asset_index = USER_ASSET_INDICES
            .load(deps.as_ref().storage, (&user_address, &ma_asset_address))
            .unwrap();
        assert_eq!(user_asset_index, expected_asset_incentive_index);

        // user gets no new rewards
        let user_unclaimed_rewards =
            USER_UNCLAIMED_REWARDS.load(deps.as_ref().storage, &user_address).unwrap();
        assert_eq!(user_unclaimed_rewards, expected_accumulated_rewards)
    }
}
