use cosmwasm_std::{
    testing::MockStorage, Addr, Decimal, OverflowError, OverflowOperation, StdError, Storage,
    Uint128,
};
use mars_incentives::{
    helpers::{compute_incentive_index, compute_user_accrued_rewards, update_incentive_index},
    state::{CONFIG, EMISSIONS, INCENTIVE_STATES},
};
use mars_red_bank_types::incentives::{Config, IncentiveState};

mod helpers;

fn store_config_with_epoch_duration(storage: &mut dyn Storage, epoch_duration: u64) {
    CONFIG
        .save(
            storage,
            &Config {
                epoch_duration,
                address_provider: Addr::unchecked(""),
                mars_denom: "".to_string(),
                min_incentive_emission: Uint128::one(),
            },
        )
        .unwrap();
}

#[test]
fn update_incentive_index_if_zero_emission() {
    let mut storage = MockStorage::default();
    let start_time = 0;
    let ai = IncentiveState {
        index: Decimal::one(),
        last_updated: 0,
    };
    INCENTIVE_STATES.save(&mut storage, ("uosmo", "umars"), &ai).unwrap();
    store_config_with_epoch_duration(&mut storage, 300);

    let current_block_time = start_time + 1;
    let mut expected_ai = ai.clone();
    expected_ai.last_updated = current_block_time;

    // only last_updated should be changed to current_block_time
    let ai = update_incentive_index(
        &mut (&storage as &dyn Storage).into(),
        "uosmo",
        "umars",
        Uint128::new(100),
        current_block_time,
    )
    .unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_incentive_index_if_zero_amount() {
    let mut storage = MockStorage::default();

    let start_time = 0;
    let ai = IncentiveState {
        index: Decimal::one(),
        last_updated: 0,
    };
    INCENTIVE_STATES.save(&mut storage, ("uosmo", "umars"), &ai).unwrap();
    store_config_with_epoch_duration(&mut storage, 300);
    EMISSIONS.save(&mut storage, ("uosmo", "umars", start_time), &Uint128::new(50)).unwrap();

    let current_block_time = start_time + 1;
    let expected_ai = ai.clone();

    // No update should occur because total_collateral is zero
    let ai = update_incentive_index(
        &mut (&storage as &dyn Storage).into(),
        "uosmo",
        "umars",
        Uint128::zero(),
        current_block_time,
    )
    .unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_incentive_index_if_current_block_lt_start_time() {
    let mut storage = MockStorage::default();

    let start_time = 10;
    let ai = IncentiveState {
        index: Decimal::one(),
        last_updated: 0,
    };
    INCENTIVE_STATES.save(&mut storage, ("uosmo", "umars"), &ai).unwrap();
    store_config_with_epoch_duration(&mut storage, 300);
    EMISSIONS.save(&mut storage, ("uosmo", "umars", start_time), &Uint128::new(50)).unwrap();

    let current_block_time = start_time - 1;
    let mut expected_ai = ai.clone();
    expected_ai.last_updated = current_block_time;

    // only last_updated should be changed to current_block_time
    let ai = update_incentive_index(
        &mut (&storage as &dyn Storage).into(),
        "uosmo",
        "umars",
        Uint128::new(100),
        current_block_time,
    )
    .unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_incentive_index_if_current_block_eq_start_time() {
    let mut storage = MockStorage::default();

    let start_time = 10;
    let ai = IncentiveState {
        index: Decimal::one(),
        last_updated: 0,
    };
    INCENTIVE_STATES.save(&mut storage, ("uosmo", "umars"), &ai).unwrap();
    store_config_with_epoch_duration(&mut storage, 300);
    EMISSIONS.save(&mut storage, ("uosmo", "umars", start_time), &Uint128::new(50)).unwrap();

    let current_block_time = start_time;
    let mut expected_ai = ai.clone();
    expected_ai.last_updated = current_block_time;

    // only last_updated should be changed to current_block_time
    let ai = update_incentive_index(
        &mut (&storage as &dyn Storage).into(),
        "uosmo",
        "umars",
        Uint128::new(100),
        current_block_time,
    )
    .unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_incentive_index_if_current_block_gt_start_time() {
    let mut storage = MockStorage::default();

    let total_amount = Uint128::new(100);

    let start_time = 10;
    let eps = Uint128::new(20);
    let ai = IncentiveState {
        index: Decimal::one(),
        last_updated: 0,
    };
    INCENTIVE_STATES.save(&mut storage, ("uosmo", "umars"), &ai).unwrap();
    store_config_with_epoch_duration(&mut storage, 300);
    EMISSIONS.save(&mut storage, ("uosmo", "umars", start_time), &eps).unwrap();

    let current_block_time = start_time + 1;
    let mut expected_ai = ai.clone();
    expected_ai.index = Decimal::from_ratio(12u128, 10u128);
    expected_ai.last_updated = current_block_time;

    let ai = update_incentive_index(
        &mut (&storage as &dyn Storage).into(),
        "uosmo",
        "umars",
        total_amount,
        current_block_time,
    )
    .unwrap();
    assert_eq!(ai, expected_ai);

    let current_block_time = current_block_time + 2;
    let mut expected_ai = ai.clone();
    expected_ai.index = Decimal::from_ratio(16u128, 10u128);
    expected_ai.last_updated = current_block_time;
    let ai = update_incentive_index(
        &mut (&storage as &dyn Storage).into(),
        "uosmo",
        "umars",
        total_amount,
        current_block_time,
    )
    .unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_incentive_index_if_last_updated_eq_end_time() {
    let mut storage = MockStorage::default();

    let start_time = 10;
    let duration = 300; // 5 min
    let end_time = start_time + duration;
    let ai = IncentiveState {
        index: Decimal::one(),
        last_updated: end_time,
    };
    INCENTIVE_STATES.save(&mut storage, ("uosmo", "umars"), &ai).unwrap();
    store_config_with_epoch_duration(&mut storage, 300);
    EMISSIONS.save(&mut storage, ("uosmo", "umars", start_time), &Uint128::new(50)).unwrap();

    let current_block_time = end_time + 1;
    let mut expected_ai = ai.clone();
    expected_ai.last_updated = current_block_time;

    // only last_updated should be changed to current_block_time
    let ai = update_incentive_index(
        &mut (&storage as &dyn Storage).into(),
        "uosmo",
        "umars",
        Uint128::new(100),
        current_block_time,
    )
    .unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_incentive_index_if_last_updated_lt_end_time() {
    let mut storage = MockStorage::default();

    let start_time = 10;
    let duration = 300; // 5 min
    let end_time = start_time + duration;
    let last_updated = end_time - 1;
    let ai = IncentiveState {
        index: Decimal::one(),
        last_updated,
    };
    INCENTIVE_STATES.save(&mut storage, ("uosmo", "umars"), &ai).unwrap();
    store_config_with_epoch_duration(&mut storage, duration);
    EMISSIONS.save(&mut storage, ("uosmo", "umars", start_time), &Uint128::new(20)).unwrap();

    let current_block_time = end_time;
    let mut expected_ai = ai.clone();
    expected_ai.index = Decimal::from_ratio(12u128, 10u128);
    expected_ai.last_updated = current_block_time;

    let ai = update_incentive_index(
        &mut (&storage as &dyn Storage).into(),
        "uosmo",
        "umars",
        Uint128::new(100),
        current_block_time,
    )
    .unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn update_incentive_index_if_not_updated_till_finished() {
    let mut storage = MockStorage::default();

    let start_time = 10;
    let duration = 300; // 5 min
    let end_time = start_time + duration;
    let ai = IncentiveState {
        index: Decimal::one(),
        last_updated: 0,
    };
    INCENTIVE_STATES.save(&mut storage, ("uosmo", "umars"), &ai).unwrap();
    store_config_with_epoch_duration(&mut storage, duration);
    EMISSIONS.save(&mut storage, ("uosmo", "umars", start_time), &Uint128::new(20)).unwrap();

    let current_block_time = end_time + 10;
    let mut expected_ai = ai.clone();
    expected_ai.index = Decimal::from_ratio(610u128, 10u128);
    expected_ai.last_updated = current_block_time;

    let ai = update_incentive_index(
        &mut (&storage as &dyn Storage).into(),
        "uosmo",
        "umars",
        Uint128::new(100),
        current_block_time,
    )
    .unwrap();
    assert_eq!(ai, expected_ai);
}

#[test]
fn test_compute_asset_incentive_index() {
    assert_eq!(
        compute_incentive_index(
            Decimal::zero(),
            Uint128::new(100),
            Uint128::new(200_000),
            1000,
            10
        ),
        Err(StdError::overflow(OverflowError::new(OverflowOperation::Sub, 1000, 10)))
    );

    assert_eq!(
        compute_incentive_index(Decimal::zero(), Uint128::new(100), Uint128::new(200_000), 0, 1000)
            .unwrap(),
        Decimal::from_ratio(1_u128, 2_u128)
    );
    assert_eq!(
        compute_incentive_index(
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
