use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    attr, coins, Addr, BankMsg, Coin, CosmosMsg, Decimal, OverflowError, OverflowOperation,
    Response, StdError, SubMsg, Timestamp, Uint128,
};

use mars_outpost::error::MarsError;
use mars_outpost::incentives::msg::{ExecuteMsg, InstantiateMsg};
use mars_outpost::incentives::AssetIncentive;
use mars_testing::{mock_dependencies, MockEnvParams};

use crate::contract::{execute, execute_balance_change, instantiate, query_user_unclaimed_rewards};
use crate::error::ContractError;
use crate::helpers::{asset_incentive_compute_index, user_compute_accrued_rewards};
use crate::state::{ASSET_INCENTIVES, CONFIG, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS};
use crate::testing::helpers::setup_test;

// init
#[test]
fn test_proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    let info = mock_info("sender", &[]);
    let msg = InstantiateMsg {
        owner: String::from("owner"),
        mars_denom: String::from("umars"),
    };

    let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    let empty_vec: Vec<SubMsg> = vec![];
    assert_eq!(empty_vec, res.messages);

    let config = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(config.owner, Addr::unchecked("owner"));
    assert_eq!(config.mars_denom, "umars".to_string());
}

// SetAssetIncentive

#[test]
fn test_only_owner_can_set_asset_incentive() {
    let mut deps = setup_test();

    let info = mock_info("sender", &[]);
    let msg = ExecuteMsg::SetAssetIncentive {
        ma_token_address: String::from("ma_asset"),
        emission_per_second: Uint128::new(100),
    };

    let res_error = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res_error, ContractError::Mars(MarsError::Unauthorized {}));
}

#[test]
fn test_set_new_asset_incentive() {
    let mut deps = setup_test();
    let ma_asset_address = Addr::unchecked("ma_asset");

    let info = mock_info("owner", &[]);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(1_000_000),
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        ma_token_address: ma_asset_address.to_string(),
        emission_per_second: Uint128::new(100),
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "set_asset_incentive"),
            attr("ma_asset", "ma_asset"),
            attr("emission_per_second", "100"),
        ]
    );

    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, &ma_asset_address).unwrap();

    assert_eq!(asset_incentive.emission_per_second, Uint128::new(100));
    assert_eq!(asset_incentive.index, Decimal::zero());
    assert_eq!(asset_incentive.last_updated, 1_000_000);
}

#[test]
fn test_set_new_asset_incentive_with_lower_and_upper_case() {
    let mut deps = setup_test();

    let ma_asset_lower_case = "ma_asset";
    let ma_asset_lower_case_addr = Addr::unchecked(ma_asset_lower_case);

    let env = mock_env();
    let info = mock_info("owner", &[]);

    // ma_token_address (lower case) should be set correctly
    {
        let msg = ExecuteMsg::SetAssetIncentive {
            ma_token_address: ma_asset_lower_case.to_string(),
            emission_per_second: Uint128::new(100),
        };

        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "set_asset_incentive"),
                attr("ma_asset", ma_asset_lower_case),
                attr("emission_per_second", "100"),
            ]
        );

        let asset_incentive =
            ASSET_INCENTIVES.load(deps.as_ref().storage, &ma_asset_lower_case_addr).unwrap();

        assert_eq!(asset_incentive.emission_per_second, Uint128::new(100));
    }

    // ma_token_address (upper case) should update asset incentive set with lower case
    // emission_per_second should be updated
    {
        deps.querier
            .set_cw20_total_supply(ma_asset_lower_case_addr.clone(), Uint128::new(2_000_000));

        let ma_asset_upper_case = ma_asset_lower_case.to_uppercase();

        let msg = ExecuteMsg::SetAssetIncentive {
            ma_token_address: ma_asset_upper_case,
            emission_per_second: Uint128::new(123),
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "set_asset_incentive"),
                attr("ma_asset", ma_asset_lower_case), // should be lower case
                attr("emission_per_second", "123"),
            ]
        );

        // asset incentive should be available with lower case address
        let asset_incentive =
            ASSET_INCENTIVES.load(deps.as_ref().storage, &ma_asset_lower_case_addr).unwrap();

        assert_eq!(asset_incentive.emission_per_second, Uint128::new(123));
    }
}

#[test]
fn test_set_existing_asset_incentive() {
    // setup
    let mut deps = setup_test();
    let ma_asset_address = Addr::unchecked("ma_asset");
    let ma_asset_total_supply = Uint128::new(2_000_000);
    deps.querier.set_cw20_total_supply(ma_asset_address.clone(), ma_asset_total_supply);

    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            &ma_asset_address,
            &AssetIncentive {
                emission_per_second: Uint128::new(100),
                index: Decimal::from_ratio(1_u128, 2_u128),
                last_updated: 500_000,
            },
        )
        .unwrap();

    // execute msg
    let info = mock_info("owner", &[]);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(1_000_000),
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        ma_token_address: ma_asset_address.to_string(),
        emission_per_second: Uint128::new(200),
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    // tests
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "set_asset_incentive"),
            attr("ma_asset", "ma_asset"),
            attr("emission_per_second", "200"),
        ]
    );

    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, &ma_asset_address).unwrap();

    let expected_index = asset_incentive_compute_index(
        Decimal::from_ratio(1_u128, 2_u128),
        Uint128::new(100),
        ma_asset_total_supply,
        500_000,
        1_000_000,
    )
    .unwrap();

    assert_eq!(asset_incentive.emission_per_second, Uint128::new(200));
    assert_eq!(asset_incentive.index, expected_index);
    assert_eq!(asset_incentive.last_updated, 1_000_000);
}

// BalanceChange

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
            attr("action", "balance_change"),
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
            attr("action", "balance_change"),
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

        let info = mock_info(&ma_asset_address.to_string(), &[]);

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
                attr("action", "balance_change"),
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
                attr("action", "balance_change"),
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
                attr("action", "balance_change"),
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

#[test]
fn test_execute_claim_rewards() {
    // SETUP
    let mut deps = setup_test();
    let user_address = Addr::unchecked("user");

    let previous_unclaimed_rewards = Uint128::new(50_000);
    let ma_asset_total_supply = Uint128::new(100_000);
    let ma_asset_user_balance = Uint128::new(10_000);
    let ma_zero_total_supply = Uint128::new(200_000);
    let ma_zero_user_balance = Uint128::new(10_000);
    let ma_no_user_total_supply = Uint128::new(100_000);
    let ma_no_user_balance = Uint128::zero();
    let time_start = 500_000_u64;
    let time_contract_call = 600_000_u64;

    // addresses
    // ma_asset with ongoing rewards
    let ma_asset_address = Addr::unchecked("ma_asset");
    // ma_asset with no pending rewards but with user index (so it had active incentives
    // at some point)
    let ma_zero_address = Addr::unchecked("ma_zero");
    // ma_asset where the user never had a balance during an active
    // incentive -> hence no associated index
    let ma_no_user_address = Addr::unchecked("ma_no_user");

    deps.querier.set_cw20_total_supply(ma_asset_address.clone(), ma_asset_total_supply);
    deps.querier.set_cw20_total_supply(ma_zero_address.clone(), ma_zero_total_supply);
    deps.querier.set_cw20_total_supply(ma_no_user_address.clone(), ma_no_user_total_supply);
    deps.querier.set_cw20_balances(
        ma_asset_address.clone(),
        &[(user_address.clone(), ma_asset_user_balance)],
    );
    deps.querier.set_cw20_balances(
        ma_zero_address.clone(),
        &[(user_address.clone(), ma_zero_user_balance)],
    );
    deps.querier.set_cw20_balances(
        ma_no_user_address.clone(),
        &[(user_address.clone(), ma_no_user_balance)],
    );

    // incentives
    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            &ma_asset_address,
            &AssetIncentive {
                emission_per_second: Uint128::new(100),
                index: Decimal::one(),
                last_updated: time_start,
            },
        )
        .unwrap();
    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            &ma_zero_address,
            &AssetIncentive {
                emission_per_second: Uint128::zero(),
                index: Decimal::one(),
                last_updated: time_start,
            },
        )
        .unwrap();
    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            &ma_no_user_address,
            &AssetIncentive {
                emission_per_second: Uint128::new(200),
                index: Decimal::one(),
                last_updated: time_start,
            },
        )
        .unwrap();

    // user indices
    USER_ASSET_INDICES
        .save(deps.as_mut().storage, (&user_address, &ma_asset_address), &Decimal::one())
        .unwrap();

    USER_ASSET_INDICES
        .save(
            deps.as_mut().storage,
            (&user_address, &ma_zero_address),
            &Decimal::from_ratio(1_u128, 2_u128),
        )
        .unwrap();

    // unclaimed_rewards
    USER_UNCLAIMED_REWARDS
        .save(deps.as_mut().storage, &user_address, &previous_unclaimed_rewards)
        .unwrap();

    let expected_ma_asset_incentive_index = asset_incentive_compute_index(
        Decimal::one(),
        Uint128::new(100),
        ma_asset_total_supply,
        time_start,
        time_contract_call,
    )
    .unwrap();

    let expected_ma_asset_accrued_rewards = user_compute_accrued_rewards(
        ma_asset_user_balance,
        Decimal::one(),
        expected_ma_asset_incentive_index,
    )
    .unwrap();

    let expected_ma_zero_accrued_rewards = user_compute_accrued_rewards(
        ma_zero_user_balance,
        Decimal::from_ratio(1_u128, 2_u128),
        Decimal::one(),
    )
    .unwrap();

    let expected_accrued_rewards = previous_unclaimed_rewards
        + expected_ma_asset_accrued_rewards
        + expected_ma_zero_accrued_rewards;

    // MSG
    let info = mock_info("user", &[]);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(time_contract_call),
        ..Default::default()
    });
    let msg = ExecuteMsg::ClaimRewards {};

    // query a bit before gives less rewards
    let env_before = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(time_contract_call - 10_000),
        ..Default::default()
    });
    let rewards_query_before =
        query_user_unclaimed_rewards(deps.as_ref(), env_before, String::from("user")).unwrap();
    assert!(rewards_query_before < expected_accrued_rewards);

    // query before execution gives expected rewards
    let rewards_query =
        query_user_unclaimed_rewards(deps.as_ref(), env.clone(), String::from("user")).unwrap();
    assert_eq!(rewards_query, expected_accrued_rewards);

    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

    // query after execution gives 0 rewards
    let rewards_query_after =
        query_user_unclaimed_rewards(deps.as_ref(), env, String::from("user")).unwrap();
    assert_eq!(rewards_query_after, Uint128::zero());

    // ASSERT

    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: user_address.to_string(),
            amount: coins(expected_accrued_rewards.u128(), "umars".to_string())
        }))]
    );

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "claim_rewards"),
            attr("user", "user"),
            attr("mars_rewards", expected_accrued_rewards),
        ]
    );

    // ma_asset and ma_zero incentives get updated, ma_no_user does not
    let ma_asset_incentive =
        ASSET_INCENTIVES.load(deps.as_ref().storage, &ma_asset_address).unwrap();
    assert_eq!(ma_asset_incentive.index, expected_ma_asset_incentive_index);
    assert_eq!(ma_asset_incentive.last_updated, time_contract_call);

    let ma_zero_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, &ma_zero_address).unwrap();
    assert_eq!(ma_zero_incentive.index, Decimal::one());
    assert_eq!(ma_zero_incentive.last_updated, time_contract_call);

    let ma_no_user_incentive =
        ASSET_INCENTIVES.load(deps.as_ref().storage, &ma_no_user_address).unwrap();
    assert_eq!(ma_no_user_incentive.index, Decimal::one());
    assert_eq!(ma_no_user_incentive.last_updated, time_start);

    // user's ma_asset and ma_zero indices are updated
    let user_ma_asset_index =
        USER_ASSET_INDICES.load(deps.as_ref().storage, (&user_address, &ma_asset_address)).unwrap();
    assert_eq!(user_ma_asset_index, expected_ma_asset_incentive_index);

    let user_ma_zero_index =
        USER_ASSET_INDICES.load(deps.as_ref().storage, (&user_address, &ma_zero_address)).unwrap();
    assert_eq!(user_ma_zero_index, Decimal::one());

    // user's ma_no_user does not get updated
    let user_ma_no_user_index = USER_ASSET_INDICES
        .may_load(deps.as_ref().storage, (&user_address, &ma_no_user_address))
        .unwrap();
    assert_eq!(user_ma_no_user_index, None);

    // user rewards are cleared
    let user_unclaimed_rewards =
        USER_UNCLAIMED_REWARDS.load(deps.as_ref().storage, &user_address).unwrap();
    assert_eq!(user_unclaimed_rewards, Uint128::zero())
}

#[test]
fn test_claim_zero_rewards() {
    // SETUP
    let mut deps = setup_test();

    let info = mock_info("user", &[]);
    let msg = ExecuteMsg::ClaimRewards {};

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(res.messages.len(), 0);
    assert_eq!(
        res.attributes,
        vec![attr("action", "claim_rewards"), attr("user", "user"), attr("mars_rewards", "0"),]
    );
}

#[test]
fn test_update_config() {
    let mut deps = setup_test();

    // *
    // non owner is not authorized
    // *
    let msg = ExecuteMsg::UpdateConfig {
        owner: None,
        mars_denom: None,
    };
    let info = mock_info("somebody", &[]);
    let error_res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(error_res, ContractError::Mars(MarsError::Unauthorized {}));

    // *
    // update config with new params
    // *
    let msg = ExecuteMsg::UpdateConfig {
        owner: Some(String::from("new_owner")),
        mars_denom: None,
    };
    let info = mock_info("owner", &[]);

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // Read config from state
    let new_config = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(new_config.owner, Addr::unchecked("new_owner"));
    assert_eq!(new_config.mars_denom, "umars".to_string());
}

#[test]
fn test_execute_cosmos_msg() {
    let mut deps = setup_test();

    let bank = BankMsg::Send {
        to_address: "destination".to_string(),
        amount: vec![Coin {
            denom: "uluna".to_string(),
            amount: Uint128::new(123456u128),
        }],
    };
    let cosmos_msg = CosmosMsg::Bank(bank);
    let msg = ExecuteMsg::ExecuteCosmosMsg(cosmos_msg.clone());

    // *
    // non owner is not authorized
    // *
    let info = mock_info("somebody", &[]);
    let error_res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
    assert_eq!(error_res, ContractError::Mars(MarsError::Unauthorized {}));

    // *
    // can execute Cosmos msg
    // *
    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert_eq!(res.messages, vec![SubMsg::new(cosmos_msg)]);
    assert_eq!(res.attributes, vec![attr("action", "execute_cosmos_msg")]);
}

#[test]
fn test_asset_incentive_compute_index() {
    assert_eq!(
        asset_incentive_compute_index(
            Decimal::zero(),
            Uint128::new(100),
            Uint128::new(200_000),
            1000,
            10
        ),
        Err(StdError::overflow(OverflowError::new(OverflowOperation::Sub, 1000, 10)))
    );

    assert_eq!(
        asset_incentive_compute_index(
            Decimal::zero(),
            Uint128::new(100),
            Uint128::new(200_000),
            0,
            1000
        )
        .unwrap(),
        Decimal::from_ratio(1_u128, 2_u128)
    );
    assert_eq!(
        asset_incentive_compute_index(
            Decimal::from_ratio(1_u128, 2_u128),
            Uint128::new(2000),
            Uint128::new(5_000_000),
            20_000,
            30_000
        )
        .unwrap(),
        Decimal::from_ratio(9_u128, 2_u128)
    );
}

#[test]
fn test_user_compute_accrued_rewards() {
    assert_eq!(
        user_compute_accrued_rewards(
            Uint128::zero(),
            Decimal::one(),
            Decimal::from_ratio(2_u128, 1_u128)
        )
        .unwrap(),
        Uint128::zero()
    );

    assert_eq!(
        user_compute_accrued_rewards(
            Uint128::new(100),
            Decimal::zero(),
            Decimal::from_ratio(2_u128, 1_u128)
        )
        .unwrap(),
        Uint128::new(200)
    );
    assert_eq!(
        user_compute_accrued_rewards(
            Uint128::new(100),
            Decimal::one(),
            Decimal::from_ratio(2_u128, 1_u128)
        )
        .unwrap(),
        Uint128::new(100)
    );
}
