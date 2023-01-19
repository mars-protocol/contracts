use cosmwasm_std::{Decimal, OverflowError, OverflowOperation, StdError, Timestamp, Uint128};
use mars_incentives::helpers::{
    compute_asset_incentive_index, compute_user_accrued_rewards, update_asset_incentive_index,
};
use mars_outpost::incentives::AssetIncentive;

mod helpers;

#[test]
fn update_asset_incentive_index_if_zero_emission() {
    let start_time = Timestamp::from_seconds(0);
    let mut ai = AssetIncentive {
        emission_per_second: Uint128::zero(),
        start_time,
        duration: 300, // 5 min
        index: Decimal::one(),
        last_updated: 0,
    };

    let current_block_time = start_time.plus_seconds(1).seconds();
    let mut expected_ai = ai.clone();
    expected_ai.last_updated = current_block_time;

    // only last_updated should be changed to current_block_time
    update_asset_incentive_index(&mut ai, Uint128::new(100), current_block_time).unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_asset_incentive_index_if_zero_amount() {
    let start_time = Timestamp::from_seconds(0);
    let mut ai = AssetIncentive {
        emission_per_second: Uint128::new(50),
        start_time,
        duration: 300, // 5 min
        index: Decimal::one(),
        last_updated: 0,
    };

    let current_block_time = start_time.plus_seconds(1).seconds();
    let mut expected_ai = ai.clone();
    expected_ai.last_updated = current_block_time;

    // only last_updated should be changed to current_block_time
    update_asset_incentive_index(&mut ai, Uint128::zero(), current_block_time).unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_asset_incentive_index_if_current_block_lt_start_time() {
    let start_time = Timestamp::from_seconds(10);
    let mut ai = AssetIncentive {
        emission_per_second: Uint128::new(50),
        start_time,
        duration: 300, // 5 min
        index: Decimal::one(),
        last_updated: 0,
    };

    let current_block_time = start_time.minus_seconds(1).seconds();
    let mut expected_ai = ai.clone();
    expected_ai.last_updated = current_block_time;

    // only last_updated should be changed to current_block_time
    update_asset_incentive_index(&mut ai, Uint128::new(100), current_block_time).unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_asset_incentive_index_if_current_block_eq_start_time() {
    let start_time = Timestamp::from_seconds(10);
    let mut ai = AssetIncentive {
        emission_per_second: Uint128::new(50),
        start_time,
        duration: 300, // 5 min
        index: Decimal::one(),
        last_updated: 0,
    };

    let current_block_time = start_time.seconds();
    let mut expected_ai = ai.clone();
    expected_ai.last_updated = current_block_time;

    // only last_updated should be changed to current_block_time
    update_asset_incentive_index(&mut ai, Uint128::new(100), current_block_time).unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_asset_incentive_index_if_current_block_gt_start_time() {
    let total_amount = Uint128::new(100);

    let start_time = Timestamp::from_seconds(10);
    let eps = Uint128::new(20);
    let mut ai = AssetIncentive {
        emission_per_second: eps,
        start_time,
        duration: 300, // 5 min
        index: Decimal::one(),
        last_updated: 0,
    };

    let current_block_time = start_time.seconds() + 1;
    let mut expected_ai = ai.clone();
    expected_ai.index = Decimal::from_ratio(12u128, 10u128);
    expected_ai.last_updated = current_block_time;

    update_asset_incentive_index(&mut ai, total_amount, current_block_time).unwrap();
    assert_eq!(ai, expected_ai);

    let current_block_time = current_block_time + 2;
    let mut expected_ai = ai.clone();
    expected_ai.index = Decimal::from_ratio(16u128, 10u128);
    expected_ai.last_updated = current_block_time;
    update_asset_incentive_index(&mut ai, total_amount, current_block_time).unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_asset_incentive_index_if_last_updated_eq_end_time() {
    let start_time = Timestamp::from_seconds(10);
    let duration = 300; // 5 min
    let end_time = start_time.plus_seconds(duration).seconds();
    let mut ai = AssetIncentive {
        emission_per_second: Uint128::new(50),
        start_time,
        duration,
        index: Decimal::one(),
        last_updated: end_time,
    };

    let current_block_time = end_time + 1;
    let mut expected_ai = ai.clone();
    expected_ai.last_updated = current_block_time;

    // only last_updated should be changed to current_block_time
    update_asset_incentive_index(&mut ai, Uint128::new(100), current_block_time).unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_asset_incentive_index_if_last_updated_gt_end_time() {
    let start_time = Timestamp::from_seconds(10);
    let duration = 300; // 5 min
    let end_time = start_time.plus_seconds(duration).seconds();
    let last_updated = end_time + 1;
    let mut ai = AssetIncentive {
        emission_per_second: Uint128::new(50),
        start_time,
        duration,
        index: Decimal::one(),
        last_updated,
    };

    let current_block_time = last_updated + 1;
    let mut expected_ai = ai.clone();
    expected_ai.last_updated = current_block_time;

    // only last_updated should be changed to current_block_time
    update_asset_incentive_index(&mut ai, Uint128::new(100), current_block_time).unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_asset_incentive_index_if_last_updated_lt_end_time() {
    let start_time = Timestamp::from_seconds(10);
    let duration = 300; // 5 min
    let end_time = start_time.plus_seconds(duration).seconds();
    let last_updated = end_time - 1;
    let mut ai = AssetIncentive {
        emission_per_second: Uint128::new(20),
        start_time,
        duration,
        index: Decimal::one(),
        last_updated,
    };

    let current_block_time = end_time;
    let mut expected_ai = ai.clone();
    expected_ai.index = Decimal::from_ratio(12u128, 10u128);
    expected_ai.last_updated = current_block_time;

    update_asset_incentive_index(&mut ai, Uint128::new(100), current_block_time).unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_asset_incentive_index_if_not_updated_till_finished() {
    let start_time = Timestamp::from_seconds(10);
    let duration = 300; // 5 min
    let end_time = start_time.plus_seconds(duration).seconds();
    let mut ai = AssetIncentive {
        emission_per_second: Uint128::new(20),
        start_time,
        duration,
        index: Decimal::one(),
        last_updated: 0,
    };

    let current_block_time = end_time + 10;
    let mut expected_ai = ai.clone();
    expected_ai.index = Decimal::from_ratio(610u128, 10u128);
    expected_ai.last_updated = current_block_time;

    update_asset_incentive_index(&mut ai, Uint128::new(100), current_block_time).unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn test_compute_asset_incentive_index() {
    assert_eq!(
        compute_asset_incentive_index(
            Decimal::zero(),
            Uint128::new(100),
            Uint128::new(200_000),
            1000,
            10
        ),
        Err(StdError::overflow(OverflowError::new(OverflowOperation::Sub, 1000, 10)))
    );

    assert_eq!(
        compute_asset_incentive_index(
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
        compute_asset_incentive_index(
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
fn test_compute_user_accrued_rewards() {
    assert_eq!(
        compute_user_accrued_rewards(
            Uint128::zero(),
            Decimal::one(),
            Decimal::from_ratio(2_u128, 1_u128)
        )
        .unwrap(),
        Uint128::zero()
    );

    assert_eq!(
        compute_user_accrued_rewards(
            Uint128::new(100),
            Decimal::zero(),
            Decimal::from_ratio(2_u128, 1_u128)
        )
        .unwrap(),
        Uint128::new(200)
    );

    assert_eq!(
        compute_user_accrued_rewards(
            Uint128::new(100),
            Decimal::one(),
            Decimal::from_ratio(2_u128, 1_u128)
        )
        .unwrap(),
        Uint128::new(100)
    );
}
