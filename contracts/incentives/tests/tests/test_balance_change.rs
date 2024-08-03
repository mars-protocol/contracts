use cosmwasm_std::{
    attr, coin,
    testing::{mock_env, mock_info},
    Addr, Decimal, Event, Response, Timestamp, Uint128,
};
use mars_incentives::{
    contract::execute,
    helpers::{compute_incentive_index, compute_user_accrued_rewards},
    mars_incentives::execute_balance_change,
    query::query_user_unclaimed_rewards,
    state::{EMISSIONS, INCENTIVE_STATES, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS},
};
use mars_testing::MockEnvParams;
use mars_types::{
    error::MarsError,
    incentives::{ExecuteMsg, IncentiveState},
    keys::{UserId, UserIdKey},
    red_bank::{Market, UserCollateralResponse},
};

use super::helpers::{th_setup, ths_setup_with_epoch_duration};

#[test]
fn balance_change_unauthorized() {
    let mut deps = th_setup();

    // the `balance_change` method can only be invoked by Red Bank contract
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake", &[]), // not Red Bank
        ExecuteMsg::BalanceChange {
            user_addr: Addr::unchecked("user"),
            account_id: None,
            denom: "uosmo".to_string(),
            user_amount_scaled_before: Uint128::new(100000),
            total_amount_scaled_before: Uint128::new(100000),
        },
    )
    .unwrap_err();
    assert_eq!(err, MarsError::Unauthorized {}.into());
}

#[test]
fn execute_balance_change_noops() {
    let mut deps = th_setup();

    // non existing incentive returns a no op
    let info = mock_info("red_bank", &[]);
    let msg = ExecuteMsg::BalanceChange {
        user_addr: Addr::unchecked("user"),
        account_id: None,
        denom: "uosmo".to_string(),
        user_amount_scaled_before: Uint128::new(100000),
        total_amount_scaled_before: Uint128::new(100000),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(
        res,
        Response::default().add_event(
            Event::new("mars/incentives/balance_change")
                .add_attribute("action", "balance_change")
                .add_attribute("denom", "uosmo")
                .add_attribute("user", "user")
        )
    )
}

#[test]
fn balance_change_zero_emission() {
    let env = mock_env();
    let mut deps = ths_setup_with_epoch_duration(env.clone(), 604800);
    let denom = "uosmo";
    let user_addr = Addr::unchecked("user");
    let asset_incentive_index = Decimal::from_ratio(1_u128, 2_u128);

    INCENTIVE_STATES
        .save(
            deps.as_mut().storage,
            (denom, "umars"),
            &IncentiveState {
                index: asset_incentive_index,
                last_updated: 500_000,
            },
        )
        .unwrap();
    EMISSIONS
        .save(deps.as_mut().storage, (denom, "umars", env.block.time.seconds()), &Uint128::zero())
        .unwrap();

    let info = mock_info("red_bank", &[]);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(600_000),
        ..Default::default()
    });
    let msg = ExecuteMsg::BalanceChange {
        user_addr: Addr::unchecked("user"),
        account_id: None,
        denom: "uosmo".to_string(),
        user_amount_scaled_before: Uint128::new(100_000),
        total_amount_scaled_before: Uint128::new(100_000),
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let expected_accrued_rewards =
        compute_user_accrued_rewards(Uint128::new(100_000), Decimal::zero(), asset_incentive_index)
            .unwrap();

    assert_eq!(
        res.events[0].attributes,
        vec![attr("action", "balance_change"), attr("denom", denom), attr("user", "user")]
    );
    assert_eq!(
        res.events[1].attributes,
        vec![
            attr("incentive_denom", "umars"),
            attr("rewards_accrued", expected_accrued_rewards),
            attr("asset_index", asset_incentive_index.to_string())
        ]
    );

    // asset incentive index stays the same
    let asset_incentive = INCENTIVE_STATES.load(deps.as_ref().storage, (denom, "umars")).unwrap();
    assert_eq!(asset_incentive.index, asset_incentive_index);
    assert_eq!(asset_incentive.last_updated, 600_000);

    let user_id = UserId::credit_manager(user_addr, "".to_string());
    let user_id_key: UserIdKey = user_id.try_into().unwrap();

    // user index is set to asset's index
    let user_asset_index =
        USER_ASSET_INDICES.load(deps.as_ref().storage, (&user_id_key, denom, "umars")).unwrap();
    assert_eq!(user_asset_index, asset_incentive_index);

    // rewards get updated
    let user_unclaimed_rewards =
        USER_UNCLAIMED_REWARDS.load(deps.as_ref().storage, (&user_id_key, denom, "umars")).unwrap();
    assert_eq!(user_unclaimed_rewards, expected_accrued_rewards)
}

#[test]
fn balance_change_user_with_zero_balance() {
    let start_index = Decimal::from_ratio(1_u128, 2_u128);
    let emission_per_second = Uint128::new(100);
    let total_supply = Uint128::new(100_000);
    let time_last_updated = 500_000_u64;
    let time_contract_call = 600_000_u64;

    let env = mock_env();
    let mut deps = ths_setup_with_epoch_duration(env, 604800);
    let denom = "uosmo";
    let user_addr = Addr::unchecked("user");

    INCENTIVE_STATES
        .save(
            deps.as_mut().storage,
            (denom, "umars"),
            &IncentiveState {
                index: start_index,
                last_updated: time_last_updated,
            },
        )
        .unwrap();
    EMISSIONS
        .save(deps.as_mut().storage, (denom, "umars", time_last_updated), &emission_per_second)
        .unwrap();

    let info = mock_info("red_bank", &[]);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(time_contract_call),
        ..Default::default()
    });
    let msg = ExecuteMsg::BalanceChange {
        user_addr: user_addr.clone(),
        account_id: None,
        denom: "uosmo".to_string(),
        user_amount_scaled_before: Uint128::zero(),
        total_amount_scaled_before: total_supply,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let expected_index = compute_incentive_index(
        start_index,
        emission_per_second,
        total_supply,
        time_last_updated,
        time_contract_call,
    )
    .unwrap();

    assert_eq!(
        res.events[0].attributes,
        vec![attr("action", "balance_change"), attr("denom", denom), attr("user", "user")]
    );
    assert_eq!(
        res.events[1].attributes,
        vec![
            attr("incentive_denom", "umars"),
            attr("rewards_accrued", "0"),
            attr("asset_index", expected_index.to_string())
        ]
    );

    // asset incentive gets updated
    let asset_incentive = INCENTIVE_STATES.load(deps.as_ref().storage, (denom, "umars")).unwrap();
    assert_eq!(asset_incentive.index, expected_index);
    assert_eq!(asset_incentive.last_updated, time_contract_call);

    let user_id = UserId::credit_manager(user_addr, "".to_string());
    let user_id_key: UserIdKey = user_id.try_into().unwrap();

    // user index is set to asset's index
    let user_asset_index =
        USER_ASSET_INDICES.load(deps.as_ref().storage, (&user_id_key, denom, "umars")).unwrap();
    assert_eq!(user_asset_index, expected_index);

    // no new rewards
    let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
        .may_load(deps.as_ref().storage, (&user_id_key, denom, "umars"))
        .unwrap();
    assert_eq!(user_unclaimed_rewards, None)
}

#[test]
fn with_zero_previous_balance_and_asset_with_zero_index_accumulates_rewards() {
    let env = mock_env();
    let mut deps = ths_setup_with_epoch_duration(env, 8640000);
    let denom = "uosmo";
    let user_addr = Addr::unchecked("user");

    let start_index = Decimal::zero();
    let emission_per_second = Uint128::new(100);
    let time_last_updated = 500_000_u64;
    let time_contract_call = 600_000_u64;

    INCENTIVE_STATES
        .save(
            deps.as_mut().storage,
            (denom, "umars"),
            &IncentiveState {
                index: start_index,
                last_updated: time_last_updated,
            },
        )
        .unwrap();
    EMISSIONS
        .save(deps.as_mut().storage, (denom, "umars", time_last_updated), &emission_per_second)
        .unwrap();

    {
        let info = mock_info("red_bank", &[]);
        let env = mars_testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call),
            ..Default::default()
        });
        let msg = ExecuteMsg::BalanceChange {
            user_addr: user_addr.clone(),
            account_id: None,
            denom: "uosmo".to_string(),
            user_amount_scaled_before: Uint128::zero(),
            total_amount_scaled_before: Uint128::zero(),
        };
        // Execute balance changed, this is the first mint of the asset, so previous total
        // supply and user balance is 0
        execute(deps.as_mut(), env, info, msg).unwrap();
    }

    {
        // Some time passes and we query the user rewards, expected value should not be 0
        let user_balance = Uint128::new(100_000);
        let total_supply = Uint128::new(100_000);
        deps.querier.set_redbank_market(Market {
            denom: denom.to_string(),
            collateral_total_scaled: total_supply,
            ..Default::default()
        });
        deps.querier.set_red_bank_user_collateral(
            &user_addr,
            UserCollateralResponse {
                denom: denom.to_string(),
                amount_scaled: user_balance,
                amount: Uint128::zero(), // doesn't matter for this test
                enabled: true,
            },
        );
        let env = mars_testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call + 1000),
            ..Default::default()
        });
        let rewards_query = query_user_unclaimed_rewards(
            deps.as_ref(),
            env,
            "user".to_string(),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        // Rewards that are accrued when no one had deposit in Red Bank are distributed to the first depositor
        assert_eq!(
            vec![coin(
                Uint128::from(time_contract_call + 1000 - time_last_updated)
                    .checked_mul(emission_per_second)
                    .unwrap()
                    .u128(),
                "umars"
            )],
            rewards_query
        );
    }
}

#[test]
fn set_new_asset_incentive_user_non_zero_balance() {
    let env = mock_env();
    let mut deps = ths_setup_with_epoch_duration(env, 8640000);
    let user_addr = Addr::unchecked("user");

    // set collateral shares for user
    let denom = "uosmo";
    let total_supply = Uint128::new(100_000);
    let user_balance = Uint128::new(10_000);

    deps.querier.set_redbank_market(Market {
        denom: denom.to_string(),
        collateral_total_scaled: total_supply,
        ..Default::default()
    });
    deps.querier.set_red_bank_user_collateral(
        &user_addr,
        UserCollateralResponse {
            denom: denom.to_string(),
            amount_scaled: user_balance,
            amount: Uint128::zero(), // doesn't matter for this test
            enabled: true,
        },
    );

    // set asset incentive
    {
        let time_last_updated = 500_000_u64;
        let emission_per_second = Uint128::new(100);
        let asset_incentive_index = Decimal::zero();

        INCENTIVE_STATES
            .save(
                deps.as_mut().storage,
                (denom, "umars"),
                &IncentiveState {
                    index: asset_incentive_index,
                    last_updated: time_last_updated,
                },
            )
            .unwrap();
        EMISSIONS
            .save(deps.as_mut().storage, (denom, "umars", time_last_updated), &emission_per_second)
            .unwrap();
    }

    // first query
    {
        let time_contract_call = 600_000_u64;

        let env = mars_testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call),
            ..Default::default()
        });

        let unclaimed_rewards = query_user_unclaimed_rewards(
            deps.as_ref(),
            env,
            "user".to_string(),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        // 100_000 s * 100 MARS/s * 1/10th of total deposit
        let expected_unclaimed_rewards = vec![coin(1_000_000, "umars")];
        assert_eq!(unclaimed_rewards, expected_unclaimed_rewards);
    }

    // increase user user deposit amount
    {
        let time_contract_call = 700_000_u64;
        let user_balance = Uint128::new(25_000);

        deps.querier.set_red_bank_user_collateral(
            &user_addr,
            UserCollateralResponse {
                denom: denom.to_string(),
                amount_scaled: user_balance,
                amount: Uint128::zero(), // doesn't matter for this test
                enabled: true,
            },
        );

        let env = mars_testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call),
            ..Default::default()
        });

        let info = mock_info("red_bank", &[]);

        execute_balance_change(
            deps.as_mut(),
            env,
            info,
            user_addr,
            None,
            denom.to_string(),
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

        let unclaimed_rewards = query_user_unclaimed_rewards(
            deps.as_ref(),
            env,
            "user".to_string(),
            None,
            None,
            None,
            None,
        )
        .unwrap();
        let expected_unclaimed_rewards = vec![coin(
            // 200_000 s * 100 MARS/s * 1/10th of total deposit +
            2_000_000 +
                // 100_000 s * 100 MARS/s * 1/4 of total deposit
                2_500_000,
            "umars",
        )];
        assert_eq!(unclaimed_rewards, expected_unclaimed_rewards);
    }
}

#[test]
fn balance_change_user_non_zero_balance() {
    let env = mock_env();
    let mut deps = ths_setup_with_epoch_duration(env, 8640000);
    let denom = "uosmo";
    let user_addr = Addr::unchecked("user");

    let emission_per_second = Uint128::new(100);
    let total_supply = Uint128::new(100_000);

    let mut expected_asset_incentive_index = Decimal::from_ratio(1_u128, 2_u128);
    let mut expected_time_last_updated = 500_000_u64;
    let mut expected_accumulated_rewards = Uint128::zero();

    INCENTIVE_STATES
        .save(
            deps.as_mut().storage,
            (denom, "umars"),
            &IncentiveState {
                index: expected_asset_incentive_index,
                last_updated: expected_time_last_updated,
            },
        )
        .unwrap();
    EMISSIONS
        .save(
            deps.as_mut().storage,
            (denom, "umars", expected_time_last_updated),
            &emission_per_second,
        )
        .unwrap();

    let info = mock_info("red_bank", &[]);

    // first call no previous rewards
    {
        let time_contract_call = 600_000_u64;
        let user_balance = Uint128::new(10_000);

        let env = mars_testing::mock_env(MockEnvParams {
            block_time: Timestamp::from_seconds(time_contract_call),
            ..Default::default()
        });
        let msg = ExecuteMsg::BalanceChange {
            user_addr: user_addr.clone(),
            account_id: None,
            denom: "uosmo".to_string(),
            user_amount_scaled_before: user_balance,
            total_amount_scaled_before: total_supply,
        };
        let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

        expected_asset_incentive_index = compute_incentive_index(
            expected_asset_incentive_index,
            emission_per_second,
            total_supply,
            expected_time_last_updated,
            time_contract_call,
        )
        .unwrap();

        let expected_accrued_rewards = compute_user_accrued_rewards(
            user_balance,
            Decimal::zero(),
            expected_asset_incentive_index,
        )
        .unwrap();
        assert_eq!(
            res.events[0].attributes,
            vec![attr("action", "balance_change"), attr("denom", denom), attr("user", "user")]
        );
        assert_eq!(
            res.events[1].attributes,
            vec![
                attr("incentive_denom", "umars"),
                attr("rewards_accrued", expected_accrued_rewards),
                attr("asset_index", expected_asset_incentive_index.to_string())
            ]
        );

        // asset incentive gets updated
        expected_time_last_updated = time_contract_call;

        let asset_incentive =
            INCENTIVE_STATES.load(deps.as_ref().storage, (denom, "umars")).unwrap();
        assert_eq!(asset_incentive.index, expected_asset_incentive_index);
        assert_eq!(asset_incentive.last_updated, expected_time_last_updated);

        let user_id = UserId::credit_manager(user_addr.clone(), "".to_string());
        let user_id_key: UserIdKey = user_id.try_into().unwrap();

        // user index is set to asset's index
        let user_asset_index =
            USER_ASSET_INDICES.load(deps.as_ref().storage, (&user_id_key, denom, "umars")).unwrap();
        assert_eq!(user_asset_index, expected_asset_incentive_index);

        // user gets new rewards
        let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
            .load(deps.as_ref().storage, (&user_id_key, denom, "umars"))
            .unwrap();
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
            user_addr: user_addr.clone(),
            account_id: None,
            denom: "uosmo".to_string(),
            user_amount_scaled_before: user_balance,
            total_amount_scaled_before: total_supply,
        };
        let res = execute(deps.as_mut(), env, info.clone(), msg).unwrap();

        let previous_user_index = expected_asset_incentive_index;
        expected_asset_incentive_index = compute_incentive_index(
            expected_asset_incentive_index,
            emission_per_second,
            total_supply,
            expected_time_last_updated,
            time_contract_call,
        )
        .unwrap();

        let expected_accrued_rewards = compute_user_accrued_rewards(
            user_balance,
            previous_user_index,
            expected_asset_incentive_index,
        )
        .unwrap();
        assert_eq!(
            res.events[0].attributes,
            vec![attr("action", "balance_change"), attr("denom", denom), attr("user", "user")]
        );
        assert_eq!(
            res.events[1].attributes,
            vec![
                attr("incentive_denom", "umars"),
                attr("rewards_accrued", expected_accrued_rewards),
                attr("asset_index", expected_asset_incentive_index.to_string())
            ]
        );

        // asset incentive gets updated
        expected_time_last_updated = time_contract_call;

        let asset_incentive =
            INCENTIVE_STATES.load(deps.as_ref().storage, (denom, "umars")).unwrap();
        assert_eq!(asset_incentive.index, expected_asset_incentive_index);
        assert_eq!(asset_incentive.last_updated, expected_time_last_updated);

        let user_id = UserId::credit_manager(user_addr.clone(), "".to_string());
        let user_id_key: UserIdKey = user_id.try_into().unwrap();

        // user index is set to asset's index
        let user_asset_index =
            USER_ASSET_INDICES.load(deps.as_ref().storage, (&user_id_key, denom, "umars")).unwrap();
        assert_eq!(user_asset_index, expected_asset_incentive_index);

        // user gets new rewards
        let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
            .load(deps.as_ref().storage, (&user_id_key, denom, "umars"))
            .unwrap();
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
            user_addr: user_addr.clone(),
            account_id: None,
            denom: "uosmo".to_string(),
            user_amount_scaled_before: user_balance,
            total_amount_scaled_before: total_supply,
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        assert_eq!(
            res.events[0].attributes,
            vec![attr("action", "balance_change"), attr("denom", denom), attr("user", "user")]
        );
        assert_eq!(
            res.events[1].attributes,
            vec![
                attr("incentive_denom", "umars"),
                attr("rewards_accrued", "0"),
                attr("asset_index", expected_asset_incentive_index.to_string())
            ]
        );

        // asset incentive is still the same
        let asset_incentive =
            INCENTIVE_STATES.load(deps.as_ref().storage, (denom, "umars")).unwrap();
        assert_eq!(asset_incentive.index, expected_asset_incentive_index);
        assert_eq!(asset_incentive.last_updated, expected_time_last_updated);

        let user_id = UserId::credit_manager(user_addr, "".to_string());
        let user_id_key: UserIdKey = user_id.try_into().unwrap();

        // user index is still the same
        let user_asset_index =
            USER_ASSET_INDICES.load(deps.as_ref().storage, (&user_id_key, denom, "umars")).unwrap();
        assert_eq!(user_asset_index, expected_asset_incentive_index);

        // user gets no new rewards
        let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
            .load(deps.as_ref().storage, (&user_id_key, denom, "umars"))
            .unwrap();
        assert_eq!(user_unclaimed_rewards, expected_accumulated_rewards)
    }
}

#[test]
fn balance_change_for_credit_account_id_with_non_zero_balance() {
    let env = mock_env();
    let mut deps = ths_setup_with_epoch_duration(env, 8640000);
    let denom = "uosmo";
    let user_addr = Addr::unchecked("credit_manager");
    let account_id = "random_account_id";

    let emission_per_second = Uint128::new(100);
    let total_supply = Uint128::new(100_000);

    let mut expected_asset_incentive_index = Decimal::from_ratio(1_u128, 2_u128);
    let mut expected_time_last_updated = 500_000_u64;
    let mut expected_accumulated_rewards = Uint128::zero();

    INCENTIVE_STATES
        .save(
            deps.as_mut().storage,
            (denom, "umars"),
            &IncentiveState {
                index: expected_asset_incentive_index,
                last_updated: expected_time_last_updated,
            },
        )
        .unwrap();
    EMISSIONS
        .save(
            deps.as_mut().storage,
            (denom, "umars", expected_time_last_updated),
            &emission_per_second,
        )
        .unwrap();

    let info = mock_info("red_bank", &[]);

    let time_contract_call = 600_000_u64;
    let user_balance = Uint128::new(10_000);

    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(time_contract_call),
        ..Default::default()
    });
    let msg = ExecuteMsg::BalanceChange {
        user_addr: user_addr.clone(),
        account_id: Some(account_id.to_string()),
        denom: "uosmo".to_string(),
        user_amount_scaled_before: user_balance,
        total_amount_scaled_before: total_supply,
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    expected_asset_incentive_index = compute_incentive_index(
        expected_asset_incentive_index,
        emission_per_second,
        total_supply,
        expected_time_last_updated,
        time_contract_call,
    )
    .unwrap();

    let expected_accrued_rewards =
        compute_user_accrued_rewards(user_balance, Decimal::zero(), expected_asset_incentive_index)
            .unwrap();
    assert_eq!(
        res.events[0].attributes,
        vec![
            attr("action", "balance_change"),
            attr("denom", denom),
            attr("user", "credit_manager"),
            attr("account_id", account_id)
        ]
    );
    assert_eq!(
        res.events[1].attributes,
        vec![
            attr("incentive_denom", "umars"),
            attr("rewards_accrued", expected_accrued_rewards),
            attr("asset_index", expected_asset_incentive_index.to_string())
        ]
    );

    // asset incentive gets updated
    expected_time_last_updated = time_contract_call;

    let asset_incentive = INCENTIVE_STATES.load(deps.as_ref().storage, (denom, "umars")).unwrap();
    assert_eq!(asset_incentive.index, expected_asset_incentive_index);
    assert_eq!(asset_incentive.last_updated, expected_time_last_updated);

    let user_id = UserId::credit_manager(user_addr, account_id.to_string());
    let user_id_key: UserIdKey = user_id.try_into().unwrap();

    // user index is set to asset's index
    let user_asset_index =
        USER_ASSET_INDICES.load(deps.as_ref().storage, (&user_id_key, denom, "umars")).unwrap();
    assert_eq!(user_asset_index, expected_asset_incentive_index);

    // user gets new rewards
    let user_unclaimed_rewards =
        USER_UNCLAIMED_REWARDS.load(deps.as_ref().storage, (&user_id_key, denom, "umars")).unwrap();
    expected_accumulated_rewards += expected_accrued_rewards;
    assert_eq!(user_unclaimed_rewards, expected_accumulated_rewards)
}
