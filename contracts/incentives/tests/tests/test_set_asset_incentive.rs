use cosmwasm_std::{
    attr, coin,
    testing::{mock_env, mock_info},
    Decimal, StdResult, Timestamp, Uint128,
};
use mars_incentives::{
    contract::execute,
    state::{EMISSIONS, INCENTIVE_STATES},
    ContractError,
};
use mars_testing::MockEnvParams;
use mars_types::{incentives::ExecuteMsg, red_bank::Market};
use mars_utils::error::ValidationError;

use super::helpers::{
    th_setup, th_setup_with_env, th_whitelist_denom, ths_setup_with_epoch_duration,
};

const ONE_WEEK_IN_SECS: u64 = 604800;

#[test]
fn invalid_denom_for_incentives() {
    let mut deps = th_setup();

    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "adfnjg&akjsfn!".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::new(100),
        start_time: 1682000400,
        duration: 2400u64,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        res,
        Err(ContractError::Validation(ValidationError::InvalidDenom {
            reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                .to_string()
        }))
    );
}

#[test]
fn cannot_set_new_asset_incentive_with_time_earlier_than_current_time() {
    let mut deps = th_setup();
    let info = mock_info("owner", &[]);
    let env = mock_env();

    th_whitelist_denom(deps.as_mut(), "umars");

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(420u128),
        start_time: env.block.time.seconds() - 1u64,
        duration: ONE_WEEK_IN_SECS,
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
fn cannot_set_new_asset_incentive_with_emission_less_than_minimum() {
    let mut deps = th_setup();
    let info = mock_info("owner", &[]);
    let env = mock_env();

    th_whitelist_denom(deps.as_mut(), "umars");

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::zero(),
        start_time: env.block.time.seconds(),
        duration: ONE_WEEK_IN_SECS,
    };
    let res_error = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert!(res_error
        .to_string()
        .starts_with("Invalid incentive: emission_per_second must be greater than min_emission:"));
}

#[test]
fn cannot_set_new_asset_incentive_with_zero_duration() {
    let mut deps = th_setup();
    let info = mock_info("owner", &[coin(0u128, "umars")]);
    let env = mock_env();

    th_whitelist_denom(deps.as_mut(), "umars");

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(1000000u32),
        start_time: env.block.time.seconds(),
        duration: 0u64,
    };
    let res_error = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(
        res_error,
        ContractError::InvalidIncentive {
            reason: "duration can't be zero".to_string()
        }
    );
}

#[test]
fn cannot_set_new_asset_incentive_with_duration_not_divisibible_by_epoch() {
    let mut deps = th_setup();
    let info = mock_info("owner", &[coin(269 * 1000000, "umars")]);
    let env = mock_env();

    th_whitelist_denom(deps.as_mut(), "umars");

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(1000000u32),
        start_time: env.block.time.seconds(),
        duration: 269u64,
    };
    let res_error = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert!(res_error.to_string().starts_with("Invalid duration. Incentive duration must be divisible by epoch duration. Epoch duration is "));
}

#[test]
fn cannot_set_new_asset_incentive_with_too_few_funds() {
    let mut deps = th_setup();
    let info = mock_info("owner", &[coin(269 * 1000000 - 1, "umars")]);
    let env = mock_env();

    th_whitelist_denom(deps.as_mut(), "umars");

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(1000000u32),
        start_time: env.block.time.seconds(),
        duration: ONE_WEEK_IN_SECS,
    };
    let res_error = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res_error.to_string(), "Invalid funds. Expected 604800000000umars funds");
}

#[test]
fn cannot_set_new_asset_incentive_with_wrong_denom() {
    let mut deps = th_setup();
    let info = mock_info("owner", &[coin(269 * 1000000, "uosmo")]);
    let env = mock_env();

    th_whitelist_denom(deps.as_mut(), "umars");

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(1000000u32),
        start_time: env.block.time.seconds(),
        duration: ONE_WEEK_IN_SECS,
    };
    let res_error = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res_error.to_string(), "Invalid funds. Expected 604800000000umars funds");
}

#[test]
fn cannot_set_new_asset_incentive_with_two_denoms() {
    let mut deps = th_setup();
    let info = mock_info("owner", &[coin(269 * 1000000, "umars"), coin(269 * 1000000, "uosmo")]);
    let env = mock_env();

    th_whitelist_denom(deps.as_mut(), "umars");

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(1000000u32),
        start_time: env.block.time.seconds(),
        duration: ONE_WEEK_IN_SECS,
    };
    let res_error = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(res_error.to_string(), "Invalid funds. Expected 604800000000umars funds");
}

#[test]
fn set_new_correct_asset_incentive_works() {
    let env = mock_env();
    let mut deps = ths_setup_with_epoch_duration(env, 604800);

    th_whitelist_denom(deps.as_mut(), "umars");

    // Setup red bank market for collateral denom
    deps.querier.set_redbank_market(Market {
        denom: "uosmo".to_string(),
        collateral_total_scaled: Uint128::from(1000000u128),
        ..Default::default()
    });

    let info = mock_info("owner", &[coin(100 * 604800, "umars")]);
    let block_time = Timestamp::from_seconds(1_000_000);
    let env = mars_testing::mock_env(MockEnvParams {
        block_time,
        ..Default::default()
    });
    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::new(100),
        start_time: block_time.seconds(),
        duration: 604800,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "set_asset_incentive"),
            attr("collateral_denom", "uosmo"),
            attr("incentive_denom", "umars"),
            attr("emission_per_second", "100"),
            attr("start_time", block_time.seconds().to_string()),
            attr("duration", "604800"),
        ]
    );

    let incentive_state = INCENTIVE_STATES.load(deps.as_ref().storage, ("uosmo", "umars")).unwrap();
    let emission_per_second =
        EMISSIONS.load(deps.as_ref().storage, ("uosmo", "umars", block_time.seconds())).unwrap();

    assert_eq!(incentive_state.index, Decimal::zero());
    assert_eq!(incentive_state.last_updated, 1_000_000);
    assert_eq!(emission_per_second, Uint128::new(100));
}

#[test]
fn can_only_set_new_incentive_with_start_time_multiple_of_epoch_duration_from_current_schedule() {
    let env = mock_env();
    let mut deps = th_setup_with_env(env.clone());

    th_whitelist_denom(deps.as_mut(), "umars");

    // Setup red bank market for collateral denom
    deps.querier.set_redbank_market(Market {
        denom: "uosmo".to_string(),
        collateral_total_scaled: Uint128::from(1000000u128),
        ..Default::default()
    });

    // Whitelist umars as incentive denom
    let msg = ExecuteMsg::UpdateWhitelist {
        add_denoms: vec![("umars".to_string(), Uint128::new(3)).into()],
        remove_denoms: vec![],
    };
    execute(deps.as_mut(), env.clone(), mock_info("owner", &[]), msg).unwrap();

    let epoch_duration = ONE_WEEK_IN_SECS;

    // First set one incentive schedule
    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(100u32),
        start_time: env.block.time.seconds(),
        duration: epoch_duration,
    };
    let funds = [coin(100 * epoch_duration as u128, "umars")];
    execute(deps.as_mut(), env.clone(), mock_info("owner", &funds), msg).unwrap();

    // Then try to set another incentive schedule with start time not multiple of epoch duration
    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(100u32),
        start_time: env.block.time.seconds() + 1,
        duration: epoch_duration,
    };
    let res_error =
        execute(deps.as_mut(), env.clone(), mock_info("owner", &funds), msg).unwrap_err();
    assert_eq!(
        res_error,
        ContractError::InvalidStartTime {
            existing_start_time: env.block.time.seconds(),
            epoch_duration,
        }
    );

    // Set another incentive schedule with start time multiple of epoch duration, should work
    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(100u32),
        start_time: env.block.time.seconds() + epoch_duration,
        duration: epoch_duration,
    };
    execute(deps.as_mut(), env, mock_info("owner", &funds), msg).unwrap();
}

#[test]
fn set_asset_incentive_merges_schedules() {
    let env = mock_env();
    let epoch_duration = ONE_WEEK_IN_SECS;
    let mut deps = ths_setup_with_epoch_duration(env.clone(), epoch_duration);

    // Setup red bank market for collateral denom
    deps.querier.set_redbank_market(Market {
        denom: "uosmo".to_string(),
        collateral_total_scaled: Uint128::from(1000000u128),
        ..Default::default()
    });

    // Whitelist umars as incentive denom
    th_whitelist_denom(deps.as_mut(), "umars");

    // First set one long schedule
    let base_eps = 100u128;
    let incentive_duration = epoch_duration * 5;
    let info = mock_info("user1", &[coin(base_eps * incentive_duration as u128, "umars")]);
    let start_time = env.block.time.seconds();
    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::new(base_eps),
        start_time,
        duration: incentive_duration,
    };
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "set_asset_incentive"),
            attr("collateral_denom", "uosmo"),
            attr("incentive_denom", "umars"),
            attr("emission_per_second", base_eps.to_string()),
            attr("start_time", start_time.to_string()),
            attr("duration", incentive_duration.to_string()),
        ]
    );

    // Read schedules to confirm. Since the added schedule spans 10 epochs, there should be 10 new
    // emission entries added to the state
    let emissions = EMISSIONS
        .range(&deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    assert!(emissions.len() == 5);
    for (i, emission) in emissions.iter().enumerate() {
        assert_eq!(
            emission.0,
            ("uosmo".to_string(), "umars".to_string(), start_time + i as u64 * epoch_duration)
        );
        assert_eq!(emission.1, Uint128::new(base_eps));
    }

    // Now set one schedule that lasts just one duration
    let incentive_duration = epoch_duration;
    let new_eps = 200u128;
    let info = mock_info("user2", &[coin(new_eps * incentive_duration as u128, "umars")]);
    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::new(new_eps),
        start_time: start_time + epoch_duration,
        duration: incentive_duration,
    };
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "set_asset_incentive"),
            attr("collateral_denom", "uosmo"),
            attr("incentive_denom", "umars"),
            attr("emission_per_second", new_eps.to_string()),
            attr("start_time", (start_time + epoch_duration).to_string()),
            attr("duration", incentive_duration.to_string()),
        ]
    );

    // Read emission entries to confirm
    let emissions = EMISSIONS
        .range(&deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    assert!(emissions.len() == 5);

    // Adding the new schedule should have incremented the emission rate of the relevant epoch, while
    // leaving the rest untouched
    assert!(emissions[0].0 .2 == start_time);
    assert!(emissions[0].1 == Uint128::new(base_eps));
    assert!(emissions[1].0 .2 == start_time + epoch_duration);
    assert!(emissions[1].1 == Uint128::new(base_eps + new_eps));
    for (i, emission) in emissions.iter().enumerate().skip(2) {
        assert!(emission.0 .2 == start_time + epoch_duration * i as u64);
        assert!(emission.1 == Uint128::new(base_eps));
    }

    // Now set a schedule that lasts for three epochs and starts one before the end of the first schedule
    let incentive_duration = epoch_duration * 3;
    let new_eps = 300u128;
    let info = mock_info("user3", &[coin(new_eps * incentive_duration as u128, "umars")]);
    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::new(new_eps),
        start_time: start_time + epoch_duration * 4,
        duration: incentive_duration,
    };
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "set_asset_incentive"),
            attr("collateral_denom", "uosmo"),
            attr("incentive_denom", "umars"),
            attr("emission_per_second", new_eps.to_string()),
            attr("start_time", (start_time + epoch_duration * 4).to_string()),
            attr("duration", incentive_duration.to_string()),
        ]
    );

    // Read emission entries to confirm
    let emissions = EMISSIONS
        .range(&deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<_>>>()
        .unwrap();
    assert!(emissions.len() == 7); // 5 + 2

    // The previous last entry should have been updated to the new emission rate, and two new entries
    // should have been added for the new schedule
    assert!(emissions[4].0 .2 == start_time + epoch_duration * 4);
    assert!(emissions[4].1 == Uint128::new(base_eps + new_eps));
    assert!(emissions[5].0 .2 == start_time + epoch_duration * 5);
    assert!(emissions[5].1 == Uint128::new(new_eps));
    assert!(emissions[6].0 .2 == start_time + epoch_duration * 6);
    assert!(emissions[6].1 == Uint128::new(new_eps));
}

#[test]
fn incorrect_denom_deposit() {
    let env = mock_env();
    let epoch_duration = ONE_WEEK_IN_SECS;
    let mut deps = ths_setup_with_epoch_duration(env.clone(), epoch_duration);

    // Test params
    let collateral_denom = "uusdc";
    let incentive_denom = "umars";
    let false_incentive_denom: &str = "ushit";
    let emission_per_second = 100u128;

    // Setup red bank market for collateral denom
    deps.querier.set_redbank_market(Market {
        denom: collateral_denom.to_string(),
        collateral_total_scaled: Uint128::from(1000000u128),
        ..Default::default()
    });

    // Whitelist umars as incentive denom
    th_whitelist_denom(deps.as_mut(), incentive_denom);

    // First set one long schedule
    let incentive_duration = epoch_duration * 5;
    let total_emissions = emission_per_second * incentive_duration as u128;
    let info = mock_info("user1", &[coin(total_emissions, false_incentive_denom)]);
    let start_time = env.block.time.seconds();
    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: collateral_denom.to_string(),
        incentive_denom: incentive_denom.to_string(),
        emission_per_second: emission_per_second.into(),
        start_time,
        duration: incentive_duration,
    };
    let err = execute(deps.as_mut(), env, info, msg).unwrap_err();

    assert_eq!(
        err,
        mars_incentives::ContractError::InvalidFunds {
            expected: coin(total_emissions, incentive_denom),
        }
    );
}
