use cosmwasm_std::{Addr, DepsMut, Empty, Env, MessageInfo, Order, Response, StdResult};
use cw2::{assert_contract_version, set_contract_version};
use cw_storage_plus::Bound;
use mars_types::{
    incentives::MigrateV1ToV2,
    keys::{UserId, UserIdKey},
};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::{MIGRATION_GUARD, OWNER, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS},
};

const FROM_VERSION: &str = "1.2.0";

pub mod v1_state {
    use cosmwasm_std::{Addr, Decimal, Uint128};
    use cw_storage_plus::Map;

    pub const USER_ASSET_INDICES: Map<(&Addr, &str, &str), Decimal> = Map::new("indices");
    pub const USER_UNCLAIMED_REWARDS: Map<(&Addr, &str, &str), Uint128> =
        Map::new("unclaimed_rewards");
}

pub fn migrate(mut deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    // Lock incentives to prevent any operations during migration.
    // Unlock is executed after full migration in `migrate_users_indexes_and_rewards`.
    MIGRATION_GUARD.try_lock(deps.storage)?;

    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    migrate_assets_indexes(&mut deps)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}

/// Migrate asset incentives indexes from V1 ASSET_INCENTIVES to V2 INCENTIVE_STATES
fn migrate_assets_indexes(deps: &mut DepsMut) -> Result<(), ContractError> {
    let asset_incentives = v1_state::USER_UNCLAIMED_REWARDS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for ((user_addr, col_denom, incentive_denom), asset_incentive) in asset_incentives.into_iter() {
        let user_id = UserId::credit_manager(user_addr, "".to_string());
        let user_id_key: UserIdKey = user_id.try_into()?;
        USER_UNCLAIMED_REWARDS.save(
            deps.storage,
            (&user_id_key, &col_denom, &incentive_denom),
            &asset_incentive,
        )?;
    }

    Ok(())
}

pub fn execute_migration(
    deps: DepsMut,
    info: MessageInfo,
    msg: MigrateV1ToV2,
) -> Result<Response, ContractError> {
    match msg {
        MigrateV1ToV2::UsersIndexesAndRewards {
            limit,
        } => migrate_users_indexes_and_rewards(deps, limit as usize),
        MigrateV1ToV2::ClearV1State {} => {
            OWNER.assert_owner(deps.storage, &info.sender)?;
            clear_v1_state(deps)
        }
    }
}

fn migrate_users_indexes_and_rewards(
    deps: DepsMut,
    limit: usize,
) -> Result<Response, ContractError> {
    // Only allow to migrate users indexes and rewards if guard is locked via `migrate` entrypoint
    MIGRATION_GUARD.assert_locked(deps.storage)?;

    // convert last key from v2 to v1
    let uai_last_key = USER_ASSET_INDICES.last(deps.storage)?.map(|kv| kv.0);
    let uai_last_key = if let Some((user_id_key, col_denom, incentive_denom)) = uai_last_key {
        let user_id: UserId = user_id_key.try_into()?;
        Some((user_id.addr, col_denom, incentive_denom))
    } else {
        None
    };

    // last key from new user asset indeces is first key (excluded) for v1 during pagination
    let start_after = uai_last_key.as_ref().map(|(addr, col_denom, incentive_denom)| {
        Bound::exclusive((addr, col_denom.as_str(), incentive_denom.as_str()))
    });
    let mut v1_uai = v1_state::USER_ASSET_INDICES
        .range(deps.storage, start_after, None, Order::Ascending)
        .take(limit + 1)
        .collect::<StdResult<Vec<_>>>()?;

    let has_more = v1_uai.len() > limit;
    if has_more {
        v1_uai.pop(); // Remove the extra item used for checking if there are more items
    }

    // save user asset indexes and unclaimed rewards
    for ((user_addr, col_denom, incentive_denom), user_asset_index) in v1_uai.into_iter() {
        let user_id = UserId::credit_manager(user_addr, "".to_string());
        let user_id_key: UserIdKey = user_id.try_into()?;
        USER_ASSET_INDICES.save(
            deps.storage,
            (&user_id_key, &col_denom, &incentive_denom),
            &user_asset_index,
        )?;
    }

    if !has_more {
        // incentives locked via `migrate` entrypoint. Unlock incentives after full migration
        MIGRATION_GUARD.try_unlock(deps.storage)?;
    }

    Ok(Response::new()
        .add_attribute("action", "migrate_users_indexes_and_rewards")
        .add_attribute(
            "result",
            if has_more {
                "in_progress"
            } else {
                "done"
            },
        )
        .add_attribute("start_after", key_to_str(uai_last_key))
        .add_attribute("limit", limit.to_string())
        .add_attribute("has_more", has_more.to_string()))
}

fn key_to_str(key: Option<(Addr, String, String)>) -> String {
    key.map(|(addr, col_denom, incentive_denom)| {
        format!("{}-{}-{}", addr, col_denom, incentive_denom)
    })
    .unwrap_or("none".to_string())
}

fn clear_v1_state(deps: DepsMut) -> Result<Response, ContractError> {
    // It is safe to clear v1 state only after full migration (guard is unlocked)
    MIGRATION_GUARD.assert_unlocked(deps.storage)?;
    v1_state::USER_ASSET_INDICES.clear(deps.storage);
    v1_state::USER_UNCLAIMED_REWARDS.clear(deps.storage);
    Ok(Response::new().add_attribute("action", "clear_v1_state"))
}
