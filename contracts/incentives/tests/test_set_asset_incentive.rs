use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{attr, Addr, Decimal, Timestamp, Uint128};

use mars_outpost::error::MarsError;
use mars_outpost::incentives::msg::ExecuteMsg;
use mars_outpost::incentives::AssetIncentive;
use mars_testing::MockEnvParams;

use mars_incentives::contract::execute;
use mars_incentives::state::ASSET_INCENTIVES;

use crate::helpers::setup_test;
use mars_incentives::helpers::asset_incentive_compute_index;
use mars_incentives::ContractError;

mod helpers;

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
            attr("action", "outposts/incentives/set_asset_incentive"),
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
                attr("action", "outposts/incentives/set_asset_incentive"),
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
                attr("action", "outposts/incentives/set_asset_incentive"),
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
            attr("action", "outposts/incentives/set_asset_incentive"),
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
