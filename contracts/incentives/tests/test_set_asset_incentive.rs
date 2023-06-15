use cosmwasm_std::{
    attr, coin,
    testing::{mock_env, mock_info},
    Decimal, Timestamp, Uint128,
};
use mars_incentives::{
    contract::execute,
    state::{INCENTIVE_SCHEDULES, INCENTIVE_STATES},
    ContractError,
};
use mars_red_bank_types::{incentives::ExecuteMsg, red_bank::Market};
use mars_testing::MockEnvParams;
use mars_utils::error::ValidationError;

use crate::helpers::th_setup;

mod helpers;

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

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(420u128),
        start_time: env.block.time.seconds() - 1u64,
        duration: 604800u64,
    };
    let res_error = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
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

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::zero(),
        start_time: env.block.time.seconds(),
        duration: 604800u64,
    };
    let res_error = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert!(res_error.to_string().starts_with(
        "Invalid incentive: emission_per_second must be greater than min_incentive_emission:"
    ));
}

#[test]
fn cannot_set_new_asset_incentive_with_zero_duration() {
    let mut deps = th_setup();
    let info = mock_info("owner", &[coin(0u128, "umars")]);
    let env = mock_env();

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(1000000u32),
        start_time: env.block.time.seconds(),
        duration: 0u64,
    };
    let res_error = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
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

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(1000000u32),
        start_time: env.block.time.seconds(),
        duration: 269u64,
    };
    let res_error = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert!(res_error.to_string().starts_with("Invalid duration. Incentive duration must be divisible by epoch duration. Epoch duration is "));
}

#[test]
fn cannot_set_new_asset_incentive_with_too_few_funds() {
    let mut deps = th_setup();
    let info = mock_info("owner", &[coin(269 * 1000000 - 1, "umars")]);
    let env = mock_env();

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(1000000u32),
        start_time: env.block.time.seconds(),
        duration: 604800u64,
    };
    let res_error = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(res_error.to_string(), "Invalid funds. Expected 604800000000 funds");
}

#[test]
fn cannot_set_new_asset_incentive_with_wrong_denom() {
    let mut deps = th_setup();
    let info = mock_info("owner", &[coin(269 * 1000000, "uosmo")]);
    let env = mock_env();

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(1000000u32),
        start_time: env.block.time.seconds(),
        duration: 604800u64,
    };
    let res_error = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(res_error.to_string(), "Invalid funds. Expected 604800000000 funds");
}

#[test]
fn cannot_set_new_asset_incentive_with_two_denoms() {
    let mut deps = th_setup();
    let info = mock_info("owner", &[coin(269 * 1000000, "umars"), coin(269 * 1000000, "uosmo")]);
    let env = mock_env();

    let msg = ExecuteMsg::SetAssetIncentive {
        collateral_denom: "uosmo".to_string(),
        incentive_denom: "umars".to_string(),
        emission_per_second: Uint128::from(1000000u32),
        start_time: env.block.time.seconds(),
        duration: 604800u64,
    };
    let res_error = execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();
    assert_eq!(res_error.to_string(), "Invalid funds. Expected 604800000000 funds");
}

#[test]
fn set_new_correct_asset_incentive_works() {
    let mut deps = th_setup();

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
    let incentive_schedule = INCENTIVE_SCHEDULES
        .load(deps.as_ref().storage, ("uosmo", "umars", block_time.seconds()))
        .unwrap();

    assert_eq!(incentive_state.index, Decimal::zero());
    assert_eq!(incentive_state.last_updated, 1_000_000);
    assert_eq!(incentive_schedule.emission_per_second, Uint128::new(100));
    assert_eq!(incentive_schedule.start_time, block_time.seconds());
    assert_eq!(incentive_schedule.duration, 604800);
}
