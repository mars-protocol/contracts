use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{
    attr,
    testing::{mock_env, mock_info},
    Addr, Decimal, Empty, Event, Order, StdResult, Uint128,
};
use cw2::{ContractVersion, VersionError};
use mars_incentives::{
    contract::{execute, migrate},
    migrations::v2_0_0::v1_state,
    state::{MIGRATION_GUARD, OWNER, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS},
    ContractError,
};
use mars_testing::mock_dependencies;
use mars_types::{
    incentives::{ExecuteMsg, MigrateV1ToV2},
    keys::{UserId, UserIdKey},
};
use mars_utils::error::GuardError;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "1.2.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

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

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "1.2.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn full_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-incentives", "1.2.0").unwrap();

    let old_owner = "spiderman_246";
    let deps_muted = deps.as_mut();
    OWNER
        .initialize(
            deps_muted.storage,
            deps_muted.api,
            mars_owner::OwnerInit::SetInitialOwner {
                owner: old_owner.to_string(),
            },
        )
        .unwrap();

    let atom_denom = "uatom";
    let usdc_denom = "uusdc";
    let osmo_denom = "uosmo";

    let mars_incentive_denom = "umars";
    let astro_incentive_denom = "uastro";

    // Set user asset indices for all incentive assets
    let user_1 = Addr::unchecked("user_1");
    let user_1_atom_idx_old = Decimal::one();
    v1_state::USER_ASSET_INDICES
        .save(
            deps.as_mut().storage,
            (&user_1, atom_denom, mars_incentive_denom),
            &user_1_atom_idx_old,
        )
        .unwrap();
    let user_1_usdc_idx_old = Decimal::from_str("2.2356").unwrap();
    v1_state::USER_ASSET_INDICES
        .save(
            deps.as_mut().storage,
            (&user_1, usdc_denom, mars_incentive_denom),
            &user_1_usdc_idx_old,
        )
        .unwrap();
    let user_1_osmo_idx_old = Decimal::from_str("33.25").unwrap();
    v1_state::USER_ASSET_INDICES
        .save(
            deps.as_mut().storage,
            (&user_1, osmo_denom, astro_incentive_denom),
            &user_1_osmo_idx_old,
        )
        .unwrap();

    // Set user asset indices only for osmo. Index is up to date with asset incentive index. No rewards accured.
    let user_2 = Addr::unchecked("user_2");
    let user_2_osmo_idx_old = Decimal::from_str("1.2356").unwrap();
    v1_state::USER_ASSET_INDICES
        .save(
            deps.as_mut().storage,
            (&user_2, osmo_denom, astro_incentive_denom),
            &user_2_osmo_idx_old,
        )
        .unwrap();

    // Set user asset indices only for atom
    let user_3 = Addr::unchecked("user_3");
    let user_3_atom_idx_old = Decimal::one();
    v1_state::USER_ASSET_INDICES
        .save(
            deps.as_mut().storage,
            (&user_3, atom_denom, mars_incentive_denom),
            &user_3_atom_idx_old,
        )
        .unwrap();

    // Set unclaimed rewards for user_2
    let user_2_usdc_mars_unclaimed_rewards = Uint128::new(500);
    v1_state::USER_UNCLAIMED_REWARDS
        .save(
            deps.as_mut().storage,
            (&user_2, usdc_denom, mars_incentive_denom),
            &user_2_usdc_mars_unclaimed_rewards,
        )
        .unwrap();
    let user_2_atom_mars_unclaimed_rewards = Uint128::new(12345);
    v1_state::USER_UNCLAIMED_REWARDS
        .save(
            deps.as_mut().storage,
            (&user_2, atom_denom, mars_incentive_denom),
            &user_2_atom_mars_unclaimed_rewards,
        )
        .unwrap();

    // Set unclaimed rewards for user_1
    let user_1_osmo_astro_unclaimed_rewards = Uint128::new(1000);
    v1_state::USER_UNCLAIMED_REWARDS
        .save(
            deps.as_mut().storage,
            (&user_1, osmo_denom, astro_incentive_denom),
            &user_1_osmo_astro_unclaimed_rewards,
        )
        .unwrap();

    // can't migrate users indexes if guard is inactive
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(old_owner, &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::UsersIndexesAndRewards {
            limit: 2,
        }),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Guard(GuardError::Inactive {}));

    let res = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "1.2.0"), attr("to_version", "2.0.0")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-incentives".to_string(),
        version: "2.0.0".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);

    // Check if user unclaimed rewards are migrated correctly
    let user_unclaimed_rewards = USER_UNCLAIMED_REWARDS
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<StdResult<HashMap<_, _>>>()
        .unwrap();
    assert_eq!(user_unclaimed_rewards.len(), 3);

    let user_id = UserId::credit_manager(user_1, "".to_string());
    let user_1_id_key: UserIdKey = user_id.try_into().unwrap();
    let user_1_osmo_astro_rewards_migrated = *user_unclaimed_rewards
        .get(&(user_1_id_key.clone(), osmo_denom.to_string(), astro_incentive_denom.to_string()))
        .unwrap();
    assert_eq!(user_1_osmo_astro_rewards_migrated, user_1_osmo_astro_unclaimed_rewards);

    let user_id = UserId::credit_manager(user_2, "".to_string());
    let user_2_id_key: UserIdKey = user_id.try_into().unwrap();
    let user_2_atom_mars_rewards_migrated = *user_unclaimed_rewards
        .get(&(user_2_id_key.clone(), atom_denom.to_string(), mars_incentive_denom.to_string()))
        .unwrap();
    assert_eq!(user_2_atom_mars_rewards_migrated, user_2_atom_mars_unclaimed_rewards);
    let user_2_usdc_mars_rewards_migrated = *user_unclaimed_rewards
        .get(&(user_2_id_key.clone(), usdc_denom.to_string(), mars_incentive_denom.to_string()))
        .unwrap();
    assert_eq!(user_2_usdc_mars_rewards_migrated, user_2_usdc_mars_unclaimed_rewards);

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

    // non-owner is unauthorized to clear state
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("random_user", &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::ClearV1State {}),
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
        mock_info("random_user_1", &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::UsersIndexesAndRewards {
            limit: 2,
        }),
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate_users_indexes_and_rewards"),
            attr("result", "in_progress"),
            attr("start_after", "user_1-uosmo-uastro"),
            attr("limit", "2"),
            attr("has_more", "true"),
        ]
    );

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("random_user_2", &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::UsersIndexesAndRewards {
            limit: 2,
        }),
    )
    .unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate_users_indexes_and_rewards"),
            attr("result", "done"),
            attr("start_after", "user_2-uosmo-uastro"),
            attr("limit", "2"),
            attr("has_more", "false"),
        ]
    );

    // check v1 state after full migration
    assert!(!v1_state::USER_ASSET_INDICES.is_empty(&deps.storage));
    assert!(!v1_state::USER_UNCLAIMED_REWARDS.is_empty(&deps.storage));

    // Check if user asset indices are updated correctly
    let user_asset_indices = USER_ASSET_INDICES
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<StdResult<HashMap<_, _>>>()
        .unwrap();
    assert_eq!(user_asset_indices.len(), 5);

    assert_eq!(
        user_asset_indices
            .get(&(user_1_id_key.clone(), atom_denom.to_string(), mars_incentive_denom.to_string()))
            .unwrap(),
        user_1_atom_idx_old
    );
    assert_eq!(
        user_asset_indices
            .get(&(user_1_id_key.clone(), usdc_denom.to_string(), mars_incentive_denom.to_string()))
            .unwrap(),
        user_1_usdc_idx_old
    );
    assert_eq!(
        user_asset_indices
            .get(&(
                user_1_id_key.clone(),
                osmo_denom.to_string(),
                astro_incentive_denom.to_string()
            ))
            .unwrap(),
        user_1_osmo_idx_old
    );

    assert_eq!(
        user_asset_indices
            .get(&(user_2_id_key, osmo_denom.to_string(), astro_incentive_denom.to_string()))
            .unwrap(),
        user_2_osmo_idx_old
    );

    let user_id = UserId::credit_manager(user_3, "".to_string());
    let user_3_id_key: UserIdKey = user_id.try_into().unwrap();
    assert_eq!(
        user_asset_indices
            .get(&(user_3_id_key.clone(), atom_denom.to_string(), mars_incentive_denom.to_string()))
            .unwrap(),
        user_3_atom_idx_old
    );

    // Clear old V1 state
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(old_owner, &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::ClearV1State {}),
    )
    .unwrap();

    // check v1 state after clearing
    assert!(v1_state::USER_ASSET_INDICES.is_empty(&deps.storage));
    assert!(v1_state::USER_UNCLAIMED_REWARDS.is_empty(&deps.storage));

    // guard should be unlocked after migration
    assert!(MIGRATION_GUARD.assert_unlocked(&deps.storage).is_ok());
}
