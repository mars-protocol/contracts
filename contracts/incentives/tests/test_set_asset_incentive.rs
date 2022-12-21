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
use mars_incentives::helpers::asset_incentive_compute_index;
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
fn test_cannot_init_asset_incentive_with_empty_params() {
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
fn test_cannot_init_asset_incentive_with_invalid_params() {
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
    let denom = "uosmo";

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

    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, denom).unwrap();

    assert_eq!(asset_incentive.emission_per_second, Uint128::new(100));
    assert_eq!(asset_incentive.index, Decimal::zero());
    assert_eq!(asset_incentive.last_updated, 1_000_000);
    assert_eq!(asset_incentive.start_time, env.block.time);
    assert_eq!(asset_incentive.duration, 86400);
}

#[test]
fn test_set_existing_asset_incentive() {
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

    let last_updated = 500_000;
    let start_time = Timestamp::from_seconds(last_updated);
    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            denom,
            &AssetIncentive {
                emission_per_second: Uint128::new(100),
                start_time,
                duration: 86400000,
                index: Decimal::from_ratio(1_u128, 2_u128),
                last_updated,
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
        denom: "uosmo".to_string(),
        emission_per_second: Some(Uint128::new(200)),
        start_time: None,
        duration: None,
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    // tests
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/incentives/set_asset_incentive"),
            attr("denom", "uosmo"),
            attr("emission_per_second", "200"),
            attr("start_time", start_time.to_string()),
            attr("duration", 86400000.to_string()),
        ]
    );

    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, denom).unwrap();

    let expected_index = asset_incentive_compute_index(
        Decimal::from_ratio(1_u128, 2_u128),
        Uint128::new(100),
        total_collateral_scaled,
        500_000,
        1_000_000,
    )
    .unwrap();

    assert_eq!(asset_incentive.emission_per_second, Uint128::new(200));
    assert_eq!(asset_incentive.index, expected_index);
    assert_eq!(asset_incentive.last_updated, 1_000_000);
    assert_eq!(asset_incentive.start_time, start_time);
    assert_eq!(asset_incentive.duration, 86400000);
}
