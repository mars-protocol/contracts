use cosmwasm_std::{
    attr,
    testing::{mock_env, mock_info},
    Decimal, Timestamp, Uint128,
};
use mars_incentives::{
    contract::execute, helpers::asset_incentive_compute_index, state::ASSET_INCENTIVES,
    ContractError,
};
use mars_outpost::{
    error::MarsError,
    incentives::{AssetIncentive, ExecuteMsg},
    red_bank::Market,
};
use mars_testing::MockEnvParams;

use crate::helpers::setup_test;

mod helpers;

#[test]
fn test_only_owner_can_set_asset_incentive() {
    let mut deps = setup_test();

    let info = mock_info("sender", &[]);
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "uosmo".to_string(),
        emission_per_second: Uint128::new(100),
    };

    let res_error = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(res_error, ContractError::Mars(MarsError::Unauthorized {}));
}

#[test]
fn test_invalid_denom_for_incentives() {
    let mut deps = setup_test();

    let info = mock_info("owner", &[]);
    let msg = ExecuteMsg::SetAssetIncentive {
        denom: "adfnjg&akjsfn!".to_string(),
        emission_per_second: Uint128::new(100),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);
    assert_eq!(
        res,
        Err(ContractError::Mars(MarsError::InvalidDenom {
            reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                .to_string()
        }))
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
        emission_per_second: Uint128::new(100),
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/incentives/set_asset_incentive"),
            attr("denom", "uosmo"),
            attr("emission_per_second", "100"),
        ]
    );

    let asset_incentive = ASSET_INCENTIVES.load(deps.as_ref().storage, denom).unwrap();

    assert_eq!(asset_incentive.emission_per_second, Uint128::new(100));
    assert_eq!(asset_incentive.index, Decimal::zero());
    assert_eq!(asset_incentive.last_updated, 1_000_000);
}

#[test]
fn test_set_existing_asset_incentive() {
    // setup
    let mut deps = setup_test();
    let denom = "uosmo";
    let total_collateral_scaled = Uint128::new(2_000_000);

    deps.querier.set_redbank_market(Market {
        denom: denom.to_string(),
        collateral_total_scaled: total_collateral_scaled,
        ..Default::default()
    });

    ASSET_INCENTIVES
        .save(
            deps.as_mut().storage,
            denom,
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
        denom: "uosmo".to_string(),
        emission_per_second: Uint128::new(200),
    };

    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    // tests
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "outposts/incentives/set_asset_incentive"),
            attr("denom", "uosmo"),
            attr("emission_per_second", "200"),
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
}
