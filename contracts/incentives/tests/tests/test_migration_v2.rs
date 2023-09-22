use std::collections::HashMap;

use cosmwasm_std::{
    attr,
    testing::{mock_env, mock_info},
    Addr, Decimal, Event, Order, StdResult, Timestamp, Uint128,
};
use cw2::{ContractVersion, VersionError};
use mars_incentives::{
    contract::{execute, migrate},
    migrations::v2_0_0::v1_state::{self, OwnerSetNoneProposed},
    state::{
        CONFIG, GUARD, INCENTIVE_STATES, OWNER, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS,
        WHITELIST, WHITELIST_COUNT,
    },
    ContractError,
};
use mars_red_bank_types::{
    incentives::{Config, ExecuteMsg, IncentiveState, MigrateMsg, MigrateV1ToV2},
    keys::{UserId, UserIdKey},
    red_bank::{Market, UserCollateralResponse},
};
use mars_testing::{mock_dependencies, MockEnvParams};
use mars_utils::error::GuardError;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "1.0.0").unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg {
            epoch_duration: 604800,
            max_whitelisted_denoms: 10,
        },
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-incentives".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-incentives", "4.1.0").unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg {
            epoch_duration: 604800,
            max_whitelisted_denoms: 10,
        },
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "1.0.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn full_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-incentives", "1.0.0").unwrap();

    let old_owner = "spiderman_246";
    v1_state::OWNER
        .save(
            deps.as_mut().storage,
            &v1_state::OwnerState::B(OwnerSetNoneProposed {
                owner: Addr::unchecked(old_owner),
            }),
        )
        .unwrap();

    let mars_denom = "umars";
    let old_config = mars_red_bank_types_old::incentives::Config {
        address_provider: Addr::unchecked("address_provider"),
        mars_denom: mars_denom.to_string(),
    };
    v1_state::CONFIG.save(deps.as_mut().storage, &old_config).unwrap();

    let atom_denom = "uatom";
    let usdc_denom = "uusdc";
    let osmo_denom = "uosmo";

    let incentive_start_time = 500_000u64;
    let duration = 864_000u64; // 10 days
    let migration_time = incentive_start_time + duration + 100u64;

    // The incentive will have to be recalculated for the entire duration
    let atom_incentive = mars_red_bank_types_old::incentives::AssetIncentive {
        emission_per_second: Uint128::new(100),
        start_time: incentive_start_time,
        duration,
        index: Decimal::one(),
        last_updated: incentive_start_time,
    };
    v1_state::ASSET_INCENTIVES.save(deps.as_mut().storage, atom_denom, &atom_incentive).unwrap();

    // The incentive will have to be recalculated for the part of the duration
    let usdc_incentive = mars_red_bank_types_old::incentives::AssetIncentive {
        emission_per_second: Uint128::new(50),
        start_time: incentive_start_time,
        duration,
        index: Decimal::from_ratio(12u128, 10u128),
        last_updated: incentive_start_time + 86400u64, // + 1 day
    };
    v1_state::ASSET_INCENTIVES.save(deps.as_mut().storage, usdc_denom, &usdc_incentive).unwrap();

    // The incentive won't be recalculated because it finished before migration time
    let osmo_incentive = mars_red_bank_types_old::incentives::AssetIncentive {
        emission_per_second: Uint128::new(50),
        start_time: incentive_start_time,
        duration,
        index: Decimal::from_ratio(15u128, 10u128),
        last_updated: migration_time - 10u64,
    };
    v1_state::ASSET_INCENTIVES.save(deps.as_mut().storage, osmo_denom, &osmo_incentive).unwrap();

    // Set user asset indices for all incentive assets
    let user_1 = Addr::unchecked("user_1");
    let user_1_atom_idx_old = Decimal::one();
    v1_state::USER_ASSET_INDICES
        .save(deps.as_mut().storage, (&user_1, atom_denom), &user_1_atom_idx_old)
        .unwrap();
    let user_1_usdc_idx_old = Decimal::one();
    v1_state::USER_ASSET_INDICES
        .save(deps.as_mut().storage, (&user_1, usdc_denom), &user_1_usdc_idx_old)
        .unwrap();
    let user_1_osmo_idx_old = Decimal::one();
    v1_state::USER_ASSET_INDICES
        .save(deps.as_mut().storage, (&user_1, osmo_denom), &user_1_osmo_idx_old)
        .unwrap();

    // Set user asset indices only for osmo. Index is up to date with asset incentive index. No rewards accured.
    let user_2 = Addr::unchecked("user_2");
    let user_2_osmo_idx_old = osmo_incentive.index;
    v1_state::USER_ASSET_INDICES
        .save(deps.as_mut().storage, (&user_2, osmo_denom), &user_2_osmo_idx_old)
        .unwrap();

    // Set user asset indices only for atom
    let user_3 = Addr::unchecked("user_3");
    let user_3_atom_idx_old = Decimal::one();
    v1_state::USER_ASSET_INDICES
        .save(deps.as_mut().storage, (&user_3, atom_denom), &user_3_atom_idx_old)
        .unwrap();

    // Set unclaimed rewards only for user_1.
    // user_2 doesn't accrue any new rewards because osmo incentive finished before migration time.
    // user_3 not set in order to check if new state creation works for him.
    let user_1_unclaimed_rewards = Uint128::new(1000);
    v1_state::USER_UNCLAIMED_REWARDS
        .save(deps.as_mut().storage, &user_1, &user_1_unclaimed_rewards)
        .unwrap();

    // Setup markets
    let atom_collateral_total_scaled = Uint128::new(100_000_000);
    deps.querier.set_redbank_market(create_market(atom_denom, atom_collateral_total_scaled));
    let usdc_collateral_total_scaled = Uint128::new(1_250_000_000);
    deps.querier.set_redbank_market(create_market(usdc_denom, usdc_collateral_total_scaled));
    let osmo_collateral_total_scaled = Uint128::new(520_000_000);
    deps.querier.set_redbank_market(create_market(osmo_denom, osmo_collateral_total_scaled));

    // Setup atom collaterals. Sum of all positions should be equal to atom_collateral_total_scaled.
    let user_1_atom_amount_scaled = Uint128::zero(); // Setting zero to check if user_1 index is updated correctly
    deps.querier.set_red_bank_user_collateral(
        &user_1,
        create_user_collateral(atom_denom, user_1_atom_amount_scaled),
    );
    let user_3_atom_amount_scaled = atom_collateral_total_scaled;
    deps.querier.set_red_bank_user_collateral(
        &user_3,
        create_user_collateral(atom_denom, user_3_atom_amount_scaled),
    );

    // Setup usdc collaterals. Sum of all positions should be equal to usdc_collateral_total_scaled
    let user_1_usdc_amount_scaled = usdc_collateral_total_scaled;
    deps.querier.set_red_bank_user_collateral(
        &user_1,
        create_user_collateral(usdc_denom, user_1_usdc_amount_scaled),
    );

    // Setup osmo collaterals. Sum of all positions should be equal to osmo_collateral_total_scaled
    let user_1_osmo_amount_scaled = Uint128::new(120_000_000);
    deps.querier.set_red_bank_user_collateral(
        &user_1,
        create_user_collateral(osmo_denom, user_1_osmo_amount_scaled),
    );
    let user_2_osmo_amount_scaled = Uint128::new(400_000_000);
    deps.querier.set_red_bank_user_collateral(
        &user_2,
        create_user_collateral(osmo_denom, user_2_osmo_amount_scaled),
    );

    let env = mars_testing::mock_env(MockEnvParams {
        block_time: Timestamp::from_seconds(migration_time),
        ..Default::default()
    });

    let epoch_duration = 604800;
    let max_whitelisted_denoms = 12;
    let res = migrate(
        deps.as_mut(),
        env,
        MigrateMsg {
            epoch_duration,
            max_whitelisted_denoms,
        },
    )
    .unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "1.0.0"), attr("to_version", "2.0.0")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-incentives".to_string(),
        version: "2.0.0".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);

    let o = OWNER.query(deps.as_ref().storage).unwrap();
    assert_eq!(old_owner.to_string(), o.owner.unwrap());
    assert!(o.proposed.is_none());
    assert!(o.initialized);
    assert!(!o.abolished);
    assert!(o.emergency_owner.is_none());

    let new_config = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        new_config,
        Config {
            address_provider: old_config.address_provider,
            max_whitelisted_denoms
        }
    );

    let whitelist_count = WHITELIST_COUNT.load(deps.as_ref().storage).unwrap();
    assert_eq!(whitelist_count, 1);
    let whitelist = WHITELIST
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<StdResult<HashMap<_, _>>>()
        .unwrap();
    assert_eq!(whitelist.len(), 1);
    assert_eq!(whitelist.get("umars").unwrap(), &Uint128::one());

    // Update asset incentive indices and check if indices changed
    let mut new_atom_incentive = atom_incentive.clone();
    v1_state::helpers::update_asset_incentive_index(
        &mut new_atom_incentive,
        atom_collateral_total_scaled,
        migration_time,
    )
    .unwrap();
    assert_ne!(atom_incentive.index, new_atom_incentive.index);
    let mut new_usdc_incentive = usdc_incentive.clone();
    v1_state::helpers::update_asset_incentive_index(
        &mut new_usdc_incentive,
        usdc_collateral_total_scaled,
        migration_time,
    )
    .unwrap();
    assert_ne!(usdc_incentive.index, new_usdc_incentive.index);
    let mut new_osmo_incentive = osmo_incentive.clone();
    v1_state::helpers::update_asset_incentive_index(
        &mut new_osmo_incentive,
        osmo_collateral_total_scaled,
        migration_time,
    )
    .unwrap();
    assert_eq!(osmo_incentive.index, new_osmo_incentive.index); // should be equal because last_updated is after incentive end time

    // Check if incentive states are updated correctly
    let incentive_states = INCENTIVE_STATES
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<StdResult<HashMap<_, _>>>()
        .unwrap();
    assert_eq!(incentive_states.len(), 3);
    assert_eq!(
        incentive_states.get(&(atom_denom.to_string(), mars_denom.to_string())).unwrap(),
        &IncentiveState {
            index: new_atom_incentive.index,
            last_updated: migration_time
        }
    );
    assert_eq!(
        incentive_states.get(&(usdc_denom.to_string(), mars_denom.to_string())).unwrap(),
        &IncentiveState {
            index: new_usdc_incentive.index,
            last_updated: migration_time
        }
    );
    assert_eq!(
        incentive_states.get(&(osmo_denom.to_string(), mars_denom.to_string())).unwrap(),
        &IncentiveState {
            index: new_osmo_incentive.index,
            last_updated: migration_time
        }
    );

    // check if guard is active for user actions
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("red-bank", &[]),
        ExecuteMsg::BalanceChange {
            user_addr: Addr::unchecked("depositor"),
            account_id: None,
            denom: "uosmo".to_string(),
            user_amount_scaled_before: Uint128::one(),
            total_amount_scaled_before: Uint128::one(),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Guard(GuardError::Active {}));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("depositor", &[]),
        ExecuteMsg::ClaimRewards {
            account_id: None,
            start_after_collateral_denom: None,
            start_after_incentive_denom: None,
            limit: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Guard(GuardError::Active {}));

    // non-owner is unauthorized to use migration
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("random_user", &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::UsersIndexesAndRewards {
            limit: 100,
            mars_denom: mars_denom.to_string(),
        }),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Owner(mars_owner::OwnerError::NotOwner {}));

    // can't clear old V1 state if migration in progress - guard is active
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(old_owner, &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::ClearV1State {}),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Guard(GuardError::Active {}));

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(old_owner, &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::UsersIndexesAndRewards {
            limit: 2,
            mars_denom: mars_denom.to_string(),
        }),
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate_users_indexes_and_rewards"),
            attr("result", "in_progress"),
            attr("start_after", "none"),
            attr("limit", "2"),
            attr("has_more", "true"),
        ]
    );

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(old_owner, &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::UsersIndexesAndRewards {
            limit: 2,
            mars_denom: mars_denom.to_string(),
        }),
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate_users_indexes_and_rewards"),
            attr("result", "in_progress"),
            attr("start_after", "user_1-uosmo"),
            attr("limit", "2"),
            attr("has_more", "true"),
        ]
    );

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(old_owner, &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::UsersIndexesAndRewards {
            limit: 2,
            mars_denom: mars_denom.to_string(),
        }),
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate_users_indexes_and_rewards"),
            attr("result", "in_progress"),
            attr("start_after", "user_2-uosmo"),
            attr("limit", "2"),
            attr("has_more", "false"),
        ]
    );

    // Check if user asset indices are updated correctly
    let user_asset_indices = USER_ASSET_INDICES
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<StdResult<HashMap<_, _>>>()
        .unwrap();
    assert_eq!(user_asset_indices.len(), 5);

    let user_id = UserId::credit_manager(user_1, "".to_string());
    let user_1_id_key: UserIdKey = user_id.try_into().unwrap();
    assert_eq!(
        user_asset_indices
            .get(&(user_1_id_key.clone(), atom_denom.to_string(), mars_denom.to_string()))
            .unwrap(),
        new_atom_incentive.index
    );
    assert_eq!(
        user_asset_indices
            .get(&(user_1_id_key.clone(), usdc_denom.to_string(), mars_denom.to_string()))
            .unwrap(),
        new_usdc_incentive.index
    );
    assert_eq!(
        user_asset_indices
            .get(&(user_1_id_key.clone(), osmo_denom.to_string(), mars_denom.to_string()))
            .unwrap(),
        new_osmo_incentive.index
    );

    let user_id = UserId::credit_manager(user_2, "".to_string());
    let user_2_id_key: UserIdKey = user_id.try_into().unwrap();
    assert_eq!(
        user_asset_indices
            .get(&(user_2_id_key, osmo_denom.to_string(), mars_denom.to_string()))
            .unwrap(),
        new_osmo_incentive.index
    );

    let user_id = UserId::credit_manager(user_3, "".to_string());
    let user_3_id_key: UserIdKey = user_id.try_into().unwrap();
    assert_eq!(
        user_asset_indices
            .get(&(user_3_id_key.clone(), atom_denom.to_string(), mars_denom.to_string()))
            .unwrap(),
        new_atom_incentive.index
    );

    // Check if user unclaimed rewards are migrated correctly
    let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<StdResult<HashMap<_, _>>>()
        .unwrap();
    assert_eq!(user_unclaimed_rewards.len(), 4);

    let user_1_atom_rewards = v1_state::helpers::compute_user_accrued_rewards(
        user_1_atom_amount_scaled,
        user_1_atom_idx_old,
        new_atom_incentive.index,
    )
    .unwrap();
    let user_1_atom_rewards_migrated = *user_unclaimed_rewards
        .get(&(user_1_id_key.clone(), atom_denom.to_string(), mars_denom.to_string()))
        .unwrap();
    assert_eq!(user_1_atom_rewards_migrated, user_1_unclaimed_rewards + user_1_atom_rewards);
    let user_1_usdc_rewards = v1_state::helpers::compute_user_accrued_rewards(
        user_1_usdc_amount_scaled,
        user_1_usdc_idx_old,
        new_usdc_incentive.index,
    )
    .unwrap();
    let user_1_usdc_rewards_migrated = *user_unclaimed_rewards
        .get(&(user_1_id_key.clone(), usdc_denom.to_string(), mars_denom.to_string()))
        .unwrap();
    assert_eq!(user_1_usdc_rewards_migrated, user_1_usdc_rewards);
    let user_1_osmo_rewards = v1_state::helpers::compute_user_accrued_rewards(
        user_1_osmo_amount_scaled,
        user_1_osmo_idx_old,
        new_osmo_incentive.index,
    )
    .unwrap();
    let user_1_osmo_rewards_migrated = *user_unclaimed_rewards
        .get(&(user_1_id_key, osmo_denom.to_string(), mars_denom.to_string()))
        .unwrap();
    assert_eq!(user_1_osmo_rewards_migrated, user_1_osmo_rewards);

    let user_2_osmo_rewards = v1_state::helpers::compute_user_accrued_rewards(
        user_2_osmo_amount_scaled,
        user_2_osmo_idx_old,
        new_osmo_incentive.index,
    )
    .unwrap();
    assert_eq!(user_2_osmo_rewards, Uint128::zero());

    let user_3_atom_rewards = v1_state::helpers::compute_user_accrued_rewards(
        user_3_atom_amount_scaled,
        user_3_atom_idx_old,
        new_atom_incentive.index,
    )
    .unwrap();
    let user_3_atom_rewards_migrated = *user_unclaimed_rewards
        .get(&(user_3_id_key, atom_denom.to_string(), mars_denom.to_string()))
        .unwrap();
    assert_eq!(user_3_atom_rewards_migrated, user_3_atom_rewards);

    // Clear old V1 state
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(old_owner, &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::ClearV1State {}),
    )
    .unwrap();

    // check users collaterals after full migration
    assert!(v1_state::ASSET_INCENTIVES.is_empty(&deps.storage));
    assert!(v1_state::USER_ASSET_INDICES.is_empty(&deps.storage));
    assert!(v1_state::USER_UNCLAIMED_REWARDS.is_empty(&deps.storage));

    // guard should be unlocked after migration
    assert!(GUARD.assert_unlocked(&deps.storage).is_ok());
}

fn create_market(denom: &str, scaled_amt: Uint128) -> Market {
    Market {
        denom: denom.to_string(),
        collateral_total_scaled: scaled_amt,
        ..Default::default()
    }
}

fn create_user_collateral(denom: &str, scaled_amt: Uint128) -> UserCollateralResponse {
    UserCollateralResponse {
        denom: denom.to_string(),
        amount_scaled: scaled_amt,
        amount: Uint128::zero(), // doesn't matter for this test
        enabled: true,
    }
}
