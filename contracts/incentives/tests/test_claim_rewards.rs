use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    attr, coins, Addr, BankMsg, CosmosMsg, Decimal, OverflowError, OverflowOperation, StdError,
    SubMsg, Timestamp, Uint128,
};

use mars_outpost::incentives::msg::ExecuteMsg;
use mars_outpost::incentives::AssetIncentive;
use mars_testing::MockEnvParams;

use mars_incentives::contract::{execute, query_user_unclaimed_rewards};
use mars_incentives::state::{ASSET_INCENTIVES, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS};

use crate::helpers::setup_test;
use mars_incentives::helpers::{asset_incentive_compute_index, user_compute_accrued_rewards};

mod helpers;

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
            attr("action", "outposts/incentives/claim_rewards"),
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
        vec![
            attr("action", "outposts/incentives/claim_rewards"),
            attr("user", "user"),
            attr("mars_rewards", "0"),
        ]
    );
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
