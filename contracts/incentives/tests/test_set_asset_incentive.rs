use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{attr, Decimal, Timestamp, Uint128};

use mars_outpost::error::MarsError;
use mars_outpost::incentives::AssetIncentive;
use mars_outpost::incentives::ExecuteMsg;
use mars_outpost::red_bank::Market;
use mars_testing::MockEnvParams;

use mars_incentives::contract::execute;
use mars_incentives::state::ASSET_INCENTIVES;

use crate::helpers::{setup_test, setup_test_with_env};
use mars_incentives::helpers::compute_asset_incentive_index;
use mars_incentives::ContractError;

mod helpers;

#[test]
fn test_only_owner_can_set_asset_incentive() {
    let mut deps = setup_test();

    let info = mock_info("sender", &[]);
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: Some(Uint128::new(100)),
        start_time: None,
        duration: Some(86400),
    };

    let res_error = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res_error, ContractError::Mars(MarsError::Unauthorized {}));
}

#[test]
fn test_cannot_set_new_asset_incentive_with_empty_params() {
    let mut deps = setup_test();
    let info = mock_info("owner", &[]);
    let env = mock_env();

    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: None,
        start_time: None,
        duration: None,
    };
    let res_error = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(
        res_error,
        ContractError::InvalidIncentive {
            reason: "all params are required during incentive initialization".to_string()
        }
    );

    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: Some(Uint128::from(100u32)),
        start_time: None,
        duration: None,
    };
    let res_error = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(
        res_error,
        ContractError::InvalidIncentive {
            reason: "all params are required during incentive initialization".to_string()
        }
    );

    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: None,
        start_time: None,
        duration: Some(2400u64),
    };
    let res_error = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        res_error,
        ContractError::InvalidIncentive {
            reason: "all params are required during incentive initialization".to_string()
        }
    );
}

#[test]
fn test_cannot_set_new_asset_incentive_with_invalid_params() {
    let mut deps = setup_test();
    let info = mock_info("owner", &[]);
    let block_time = Timestamp::from_seconds(1_000_000);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time,
        ..Default::default()
    });

    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: Some(Uint128::from(100u32)),
        start_time: None,
        duration: Some(0u64),
    };
    let res_error = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(
        res_error,
        ContractError::InvalidIncentive {
            reason: "duration can't be 0".to_string()
        }
    );

    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: Some(Uint128::from(100u32)),
        start_time: Some(block_time.minus_seconds(1u64)),
        duration: Some(100u64),
    };
    let res_error = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        res_error,
        ContractError::InvalidIncentive {
            reason: "start_time can't be less than current block time".to_string()
        }
    );
}

#[test]
fn test_set_new_asset_incentive() {
    let mut deps = setup_test();

    let info = mock_info("owner", &[]);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(1_000_000),
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: Some(Uint128::new(100)),
        start_time: None,
        duration: Some(86400),
    };

    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/incentives/set_asset_incentive"),
            attr("denom", "uosmo"),
            attr("emission_per_second", "100"),
            attr("start_time", env.block.time.clone().to_string()),
            attr("duration", "86400"),
        ]
    );

    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, "uosmo").unwrap();

    assert_eq!(asset_incentive.emission_per_second, Uint128::new(100));
    assert_eq!(asset_incentive.index, Decimal::zero());
    assert_eq!(asset_incentive.last_updated, 1_000_000);
    assert_eq!(asset_incentive.start_time, env.block.time);
    assert_eq!(asset_incentive.duration, 86400);
}

#[test]
fn test_set_existing_asset_incentive_with_different_start_time() {
    let mut deps = setup_test();

    deps.querier.set_redbank_market(Market {
        denom: "uosmo".to_string(),
        collateral_total_scaled: Uint128::new(100_000),
        ..Default::default()
    });

    let info = mock_info("owner", &[]);

    let start_time = Timestamp::from_seconds(1_000_000);
    let duration = 300_000;
    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            "uosmo",
            &AssetIncentive {
                emission_per_second: Uint128::new(124),
                start_time,
                duration,
                index: Decimal::zero(),
                last_updated: start_time.seconds(),
            },
        )
        .unwrap();

    // can't modify start_time if incentive in progress
    let block_time = start_time.plus_seconds(duration);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time,
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: None,
        start_time: Some(block_time.plus_seconds(10)),
        duration: None,
    };
    let res_error = execute(deps.as_mut(), env, info.clone(), msg).unwrap_err();
    assert_eq!(
        res_error,
        ContractError::InvalidIncentive {
            reason: "can't modify start_time if incentive in progress".to_string()
        }
    );

    // start_time can't be less than current block time
    let block_time = start_time.plus_seconds(duration).plus_seconds(1);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time,
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: None,
        start_time: Some(block_time.minus_seconds(1)),
        duration: None,
    };
    let res_error = execute(deps.as_mut(), env, info.clone(), msg).unwrap_err();
    assert_eq!(
        res_error,
        ContractError::InvalidIncentive {
            reason: "start_time can't be less than current block time".to_string()
        }
    );

    // set new start_time
    let block_time = start_time.plus_seconds(duration).plus_seconds(1);
    let start_time = block_time.plus_seconds(10);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time,
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: None,
        start_time: Some(start_time),
        duration: None,
    };
    execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, "uosmo").unwrap();
    assert_eq!(asset_incentive.start_time, start_time);
    assert_eq!(asset_incentive.last_updated, block_time.seconds());

    // start time can be set to current block time only if previous incentive has finished
    let block_time = start_time.plus_seconds(duration).plus_seconds(1);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time,
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: None,
        start_time: None,
        duration: None,
    };
    execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, "uosmo").unwrap();
    assert_eq!(asset_incentive.start_time, block_time);
    assert_eq!(asset_incentive.last_updated, block_time.seconds());

    // incentive in progress, leave previous start_time
    let block_time = start_time.plus_seconds(duration);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time,
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: None,
        start_time: None,
        duration: None,
    };
    let prev_asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, "uosmo").unwrap();
    execute(deps.as_mut(), env, info, msg).unwrap();
    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, "uosmo").unwrap();
    assert_eq!(asset_incentive.start_time, prev_asset_incentive.start_time);
    assert_eq!(asset_incentive.last_updated, block_time.seconds());
}

#[test]
fn test_set_existing_asset_incentive_with_different_duration() {
    let mut deps = setup_test();

    deps.querier.set_redbank_market(Market {
        denom: "uosmo".to_string(),
        collateral_total_scaled: Uint128::new(100_000),
        ..Default::default()
    });

    let info = mock_info("owner", &[]);

    let start_time = Timestamp::from_seconds(1_000_000);
    let duration = 300_000;
    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            "uosmo",
            &AssetIncentive {
                emission_per_second: Uint128::new(124),
                start_time,
                duration,
                index: Decimal::zero(),
                last_updated: start_time.seconds(),
            },
        )
        .unwrap();

    // duration can't be 0
    let block_time = start_time.plus_seconds(duration);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time,
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: None,
        start_time: None,
        duration: Some(0),
    };
    let res_error = execute(deps.as_mut(), env, info.clone(), msg).unwrap_err();
    assert_eq!(
        res_error,
        ContractError::InvalidIncentive {
            reason: "duration can't be 0".to_string()
        }
    );

    // end_time can't be less than current block time (can't decrease duration of the incentive)
    let block_time = start_time.plus_seconds(duration);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time,
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: None,
        start_time: None,
        duration: Some(duration - 1),
    };
    let res_error = execute(deps.as_mut(), env, info.clone(), msg).unwrap_err();
    assert_eq!(
        res_error,
        ContractError::InvalidIncentive {
            reason: "end_time can't be less than current block time".to_string()
        }
    );

    // increase duration of the incentive
    let block_time = start_time.plus_seconds(duration);
    let duration = duration + 10;
    let env = mars_testing::mock_env(MockEnvParams {
        block_time,
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: None,
        start_time: None,
        duration: Some(duration),
    };
    let prev_asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, "uosmo").unwrap();
    execute(deps.as_mut(), env, info.clone(), msg).unwrap();
    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, "uosmo").unwrap();
    assert_eq!(asset_incentive.start_time, prev_asset_incentive.start_time);
    assert_eq!(asset_incentive.duration, duration);
    assert_eq!(asset_incentive.last_updated, block_time.seconds());

    // leave prev duration
    let block_time = start_time.plus_seconds(duration);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time,
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: Some(Uint128::new(300)),
        start_time: None,
        duration: None,
    };
    let prev_asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, "uosmo").unwrap();
    execute(deps.as_mut(), env, info, msg).unwrap();
    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, "uosmo").unwrap();
    assert_eq!(asset_incentive.emission_per_second, Uint128::new(300));
    assert_eq!(asset_incentive.start_time, prev_asset_incentive.start_time);
    assert_eq!(asset_incentive.duration, prev_asset_incentive.duration);
    assert_eq!(asset_incentive.last_updated, block_time.seconds());
}

#[test]
fn test_set_existing_asset_incentive_with_index_updated_during_incentive() {
    // setup
    let env = mock_env();
    let mut deps = setup_test_with_env(env);
    let denom = "uosmo";
    let total_collateral_scaled = Uint128::new(2_000_000);

    deps.querier.set_redbank_market(Market {
        denom: denom.to_string(),
        collateral_total_scaled: total_collateral_scaled,
        ..Default::default()
    });

    let start_time = Timestamp::from_seconds(500_000);
    let last_updated = start_time.minus_seconds(10);
    let duration = 86400000;
    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            denom,
            &AssetIncentive {
                emission_per_second: Uint128::new(100),
                start_time,
                duration,
                index: Decimal::from_ratio(1_u128, 2_u128),
                last_updated: last_updated.seconds(),
            },
        )
        .unwrap();

    // update emission and current index when (current_block_time >= asset_incentive.start_time && asset_incentive.last_updated < asset_incentive.start_time)
    let info = mock_info("owner", &[]);
    let block_time = Timestamp::from_seconds(1_000_000);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time,
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: denom.to_string(),
        emission_per_second: Some(Uint128::new(200)),
        start_time: None,
        duration: None,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/incentives/set_asset_incentive"),
            attr("denom", denom),
            attr("emission_per_second", "200"),
            attr("start_time", start_time.to_string()),
            attr("duration", duration.to_string()),
        ]
    );

    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, denom).unwrap();

    let expected_index = compute_asset_incentive_index(
        Decimal::from_ratio(1_u128, 2_u128),
        Uint128::new(100),
        total_collateral_scaled,
        start_time.seconds(),
        block_time.seconds(),
    )
    .unwrap();

    assert_eq!(asset_incentive.emission_per_second, Uint128::new(200));
    assert_eq!(asset_incentive.start_time, start_time);
    assert_eq!(asset_incentive.duration, duration);
    assert_eq!(asset_incentive.index, expected_index);
    assert_eq!(asset_incentive.last_updated, block_time.seconds());
}

#[test]
fn test_set_existing_asset_incentive_with_index_updated_after_incentive() {
    // setup
    let env = mock_env();
    let mut deps = setup_test_with_env(env);
    let denom = "uosmo";
    let total_collateral_scaled = Uint128::new(2_000_000);

    deps.querier.set_redbank_market(Market {
        denom: denom.to_string(),
        collateral_total_scaled: total_collateral_scaled,
        ..Default::default()
    });

    let start_time = Timestamp::from_seconds(500_000);
    let last_updated = start_time.plus_seconds(10);
    let duration = 120000;
    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            denom,
            &AssetIncentive {
                emission_per_second: Uint128::new(120),
                start_time,
                duration,
                index: Decimal::from_ratio(1_u128, 4_u128),
                last_updated: last_updated.seconds(),
            },
        )
        .unwrap();

    // update current index when (current_block_time >= asset_incentive.end_time && asset_incentive.last_updated < asset_incentive.end_time)
    let info = mock_info("owner", &[]);
    let block_time = start_time.plus_seconds(duration).plus_seconds(1);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time,
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: denom.to_string(),
        emission_per_second: Some(Uint128::new(215)),
        start_time: None,
        duration: None,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/incentives/set_asset_incentive"),
            attr("denom", denom),
            attr("emission_per_second", "215"),
            attr("start_time", block_time.to_string()),
            attr("duration", duration.to_string()),
        ]
    );

    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, denom).unwrap();

    let expected_index = compute_asset_incentive_index(
        Decimal::from_ratio(1_u128, 4_u128),
        Uint128::new(120),
        total_collateral_scaled,
        last_updated.seconds(),
        start_time.plus_seconds(duration).seconds(),
    )
    .unwrap();

    assert_eq!(asset_incentive.emission_per_second, Uint128::new(215));
    assert_eq!(asset_incentive.start_time, block_time);
    assert_eq!(asset_incentive.duration, duration);
    assert_eq!(asset_incentive.index, expected_index);
    assert_eq!(asset_incentive.last_updated, block_time.seconds());
}
