use cosmwasm_std::{Addr, DepsMut, MessageInfo, Order, Response, StdResult};
use cw2::{assert_contract_version, set_contract_version};
use cw_storage_plus::Bound;
use mars_types::{
    keys::{UserId, UserIdKey},
    red_bank::{Config, Market, MigrateV1ToV2},
};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::{COLLATERALS, CONFIG, MARKETS, MIGRATION_GUARD, OWNER},
};

const FROM_VERSION: &str = "1.2.1";

pub mod v1_state {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Decimal, Uint128};
    use cw_storage_plus::{Item, Map};
    use mars_types::red_bank::{Collateral, InterestRateModel};

    pub const CONFIG: Item<Config> = Item::new("config");
    pub const MARKETS: Map<&str, Market> = Map::new("markets");
    pub const COLLATERALS: Map<(&Addr, &str), Collateral> = Map::new("collaterals");

    #[cw_serde]
    pub struct Config {
        pub address_provider: Addr,
        pub close_factor: Decimal,
    }

    #[cw_serde]
    pub struct Market {
        pub denom: String,
        pub max_loan_to_value: Decimal,
        pub liquidation_threshold: Decimal,
        pub liquidation_bonus: Decimal,
        pub reserve_factor: Decimal,
        pub interest_rate_model: InterestRateModel,
        pub borrow_index: Decimal,
        pub liquidity_index: Decimal,
        pub borrow_rate: Decimal,
        pub liquidity_rate: Decimal,
        pub indexes_last_updated: u64,
        pub collateral_total_scaled: Uint128,
        pub debt_total_scaled: Uint128,
        pub deposit_enabled: bool,
        pub borrow_enabled: bool,
        pub deposit_cap: Uint128,
    }
}

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    // Lock red-bank to prevent any operations during migration.
    // Unlock is executed after full migration in `migrate_collaterals`.
    MIGRATION_GUARD.try_lock(deps.storage)?;

    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    // Config package updated, re-initializing
    let old_config = v1_state::CONFIG.load(deps.storage)?;
    v1_state::CONFIG.remove(deps.storage);
    CONFIG.save(
        deps.storage,
        &Config {
            address_provider: old_config.address_provider,
        },
    )?;

    // Migrate markets.
    // Remove LP tokens because they are not supported in red-bank. Params for LP tokens exist in `params`contract.
    let markets = v1_state::MARKETS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    v1_state::MARKETS.clear(deps.storage);
    for (denom, market) in markets.into_iter() {
        if denom.starts_with("gamm/pool") {
            continue;
        }
        MARKETS.save(deps.storage, &denom, &market.into())?;
    }

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}

/// Few of the fields in old `Market` struct are moved to `params` contract
impl From<v1_state::Market> for Market {
    fn from(value: v1_state::Market) -> Self {
        Self {
            denom: value.denom,
            interest_rate_model: value.interest_rate_model,
            borrow_index: value.borrow_index,
            liquidity_index: value.liquidity_index,
            borrow_rate: value.borrow_rate,
            liquidity_rate: value.liquidity_rate,
            indexes_last_updated: value.indexes_last_updated,
            collateral_total_scaled: value.collateral_total_scaled,
            debt_total_scaled: value.debt_total_scaled,
            reserve_factor: value.reserve_factor,
        }
    }
}

pub fn execute_migration(
    deps: DepsMut,
    info: MessageInfo,
    msg: MigrateV1ToV2,
) -> Result<Response, ContractError> {
    match msg {
        MigrateV1ToV2::Collaterals {
            limit,
        } => migrate_collaterals(deps, limit as usize),
        MigrateV1ToV2::ClearV1State {} => {
            OWNER.assert_owner(deps.storage, &info.sender)?;
            clear_v1_state(deps)
        }
    }
}

fn migrate_collaterals(deps: DepsMut, limit: usize) -> Result<Response, ContractError> {
    // Only allow to migrate collaterals if guard is locked via `migrate` entrypoint
    MIGRATION_GUARD.assert_locked(deps.storage)?;

    // convert last key from v2 to v1
    let colls_last_key = COLLATERALS.last(deps.storage)?.map(|kv| kv.0);
    let colls_last_key = if let Some((user_id_key, denom)) = colls_last_key {
        let user_id: UserId = user_id_key.try_into()?;
        Some((user_id.addr, denom))
    } else {
        None
    };

    // last key from new collaterals is first key (excluded) for v1 collaterals during pagination
    let start_after =
        colls_last_key.as_ref().map(|(addr, denom)| Bound::exclusive((addr, denom.as_str())));
    let mut v1_colls = v1_state::COLLATERALS
        .range(deps.storage, start_after, None, Order::Ascending)
        .take(limit + 1)
        .collect::<StdResult<Vec<_>>>()?;

    let has_more = v1_colls.len() > limit;
    if has_more {
        v1_colls.pop(); // Remove the extra item used for checking if there are more items
    }

    // save collaterals
    for ((user_addr, denom), collateral) in v1_colls.into_iter() {
        let user_id = UserId::credit_manager(user_addr, "".to_string());
        let user_id_key: UserIdKey = user_id.try_into()?;
        COLLATERALS.save(deps.storage, (&user_id_key, &denom), &collateral)?;
    }

    if !has_more {
        // red-bank locked via `migrate` entrypoint. Unlock red-bank after full migration
        MIGRATION_GUARD.try_unlock(deps.storage)?;
    }

    Ok(Response::new()
        .add_attribute("action", "migrate_collaterals")
        .add_attribute(
            "result",
            if has_more {
                "in_progress"
            } else {
                "done"
            },
        )
        .add_attribute("start_after", key_to_str(colls_last_key))
        .add_attribute("limit", limit.to_string())
        .add_attribute("has_more", has_more.to_string()))
}

fn key_to_str(key: Option<(Addr, String)>) -> String {
    key.map(|(addr, denom)| format!("{}-{}", addr, denom)).unwrap_or("none".to_string())
}

fn clear_v1_state(deps: DepsMut) -> Result<Response, ContractError> {
    // It is safe to clear v1 state only after full migration (guard is unlocked)
    MIGRATION_GUARD.assert_unlocked(deps.storage)?;
    v1_state::COLLATERALS.clear(deps.storage);
    Ok(Response::new().add_attribute("action", "clear_v1_state"))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use cosmwasm_std::{attr, testing::mock_dependencies, Addr, Uint128};
    use mars_types::red_bank::Collateral;
    use mars_utils::error::GuardError;

    use super::*;

    #[test]
    fn cannot_migrate_v1_collaterals_without_lock() {
        let mut deps = mock_dependencies();

        let res_error = migrate_collaterals(deps.as_mut(), 10).unwrap_err();
        assert_eq!(res_error, ContractError::Guard(GuardError::Inactive {}));
    }

    #[test]
    fn empty_v1_collaterals() {
        let mut deps = mock_dependencies();

        MIGRATION_GUARD.try_lock(deps.as_mut().storage).unwrap();

        let res = migrate_collaterals(deps.as_mut(), 10).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_collaterals"),
                attr("result", "done"),
                attr("start_after", "none"),
                attr("limit", "10"),
                attr("has_more", "false"),
            ]
        );
    }

    #[test]
    fn migrate_v1_collaterals() {
        let mut deps = mock_dependencies();

        MIGRATION_GUARD.try_lock(deps.as_mut().storage).unwrap();

        // prepare v1 collaterals
        let user_1_osmo_collateral = Collateral {
            amount_scaled: Uint128::new(12345),
            enabled: true,
        };
        v1_state::COLLATERALS
            .save(
                deps.as_mut().storage,
                (&Addr::unchecked("user_1"), "uosmo"),
                &user_1_osmo_collateral,
            )
            .unwrap();
        let user_2_atom_collateral = Collateral {
            amount_scaled: Uint128::new(1),
            enabled: true,
        };
        v1_state::COLLATERALS
            .save(
                deps.as_mut().storage,
                (&Addr::unchecked("user_2"), "uatom"),
                &user_2_atom_collateral,
            )
            .unwrap();
        let user_2_jake_collateral = Collateral {
            amount_scaled: Uint128::new(1023),
            enabled: true,
        };
        v1_state::COLLATERALS
            .save(
                deps.as_mut().storage,
                (&Addr::unchecked("user_2"), "ujake"),
                &user_2_jake_collateral,
            )
            .unwrap();
        let user_3_jake_collateral = Collateral {
            amount_scaled: Uint128::new(1111111),
            enabled: false,
        };
        v1_state::COLLATERALS
            .save(
                deps.as_mut().storage,
                (&Addr::unchecked("user_3"), "ujake"),
                &user_3_jake_collateral,
            )
            .unwrap();
        let user_1_axl_collateral = Collateral {
            amount_scaled: Uint128::new(123456789),
            enabled: true,
        };
        v1_state::COLLATERALS
            .save(
                deps.as_mut().storage,
                (&Addr::unchecked("user_1"), "uaxl"),
                &user_1_axl_collateral,
            )
            .unwrap();

        // migrate first 2 collaterals
        let res = migrate_collaterals(deps.as_mut(), 2).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_collaterals"),
                attr("result", "in_progress"),
                attr("start_after", "none"),
                attr("limit", "2"),
                attr("has_more", "true"),
            ]
        );

        // in new collaterals we should have 2 collaterals
        let collaterals = COLLATERALS
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<HashMap<_, _>>>()
            .unwrap();
        assert_eq!(collaterals.len(), 2);

        // migrate next 2 collaterals
        let res = migrate_collaterals(deps.as_mut(), 2).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_collaterals"),
                attr("result", "in_progress"),
                attr("start_after", "user_1-uosmo"),
                attr("limit", "2"),
                attr("has_more", "true"),
            ]
        );

        // in new collaterals we should have more 2 collaterals, 4 in total
        let collaterals = COLLATERALS
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<HashMap<_, _>>>()
            .unwrap();
        assert_eq!(collaterals.len(), 4);

        // migrate next 2 collaterals, we have only 1 left
        let res = migrate_collaterals(deps.as_mut(), 2).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_collaterals"),
                attr("result", "done"),
                attr("start_after", "user_2-ujake"),
                attr("limit", "2"),
                attr("has_more", "false"),
            ]
        );

        // in new collaterals we should have more 1 collaterals, 5 in total
        let collaterals = COLLATERALS
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<HashMap<_, _>>>()
            .unwrap();
        assert_eq!(collaterals.len(), 5);

        // compare values
        let user_id = UserId::credit_manager(Addr::unchecked("user_1"), "".to_string());
        let user_1_id_key: UserIdKey = user_id.try_into().unwrap();
        assert_eq!(
            collaterals.get(&(user_1_id_key.clone(), "uosmo".to_string())).unwrap(),
            &user_1_osmo_collateral
        );
        assert_eq!(
            collaterals.get(&(user_1_id_key, "uaxl".to_string())).unwrap(),
            &user_1_axl_collateral
        );
        let user_id = UserId::credit_manager(Addr::unchecked("user_2"), "".to_string());
        let user_2_id_key: UserIdKey = user_id.try_into().unwrap();
        assert_eq!(
            collaterals.get(&(user_2_id_key.clone(), "uatom".to_string())).unwrap(),
            &user_2_atom_collateral
        );
        assert_eq!(
            collaterals.get(&(user_2_id_key, "ujake".to_string())).unwrap(),
            &user_2_jake_collateral
        );
        let user_id = UserId::credit_manager(Addr::unchecked("user_3"), "".to_string());
        let user_3_id_key: UserIdKey = user_id.try_into().unwrap();
        assert_eq!(
            collaterals.get(&(user_3_id_key, "ujake".to_string())).unwrap(),
            &user_3_jake_collateral
        );

        // try to migrate one more time, guard is unlocked
        let res_err = migrate_collaterals(deps.as_mut(), 2).unwrap_err();
        assert_eq!(res_err, ContractError::Guard(GuardError::Inactive {}));
    }
}
