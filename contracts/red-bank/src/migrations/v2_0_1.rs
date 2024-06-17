use cosmwasm_std::{Addr, DepsMut, MessageInfo, Order, Response, StdResult};
use cw2::{assert_contract_version, set_contract_version};
use cw_storage_plus::Bound;
use mars_types::{
    address_provider::{AddressResponseItem, MarsAddressType, QueryMsg as AddressProviderQueryMsg},
    keys::{UserId, UserIdKey},
    red_bank::{Debt, MigrateV2ToV2_0_1},
};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::{CONFIG, DEBTS, MIGRATION_GUARD, OWNER},
};

const FROM_VERSION: &str = "2.0.0";

pub mod v2_state {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Uint128};
    use cw_storage_plus::Map;
    use mars_types::red_bank::Debt;
    pub const DEBTS: Map<(&Addr, &str), Debt> = Map::new("debts");

    #[cw_serde]
    pub struct SharesResponseItem {
        pub account_id: String,
        pub denom: String,
        pub shares: Uint128,
    }

    #[cw_serde]
    pub struct DebtShares {
        pub denom: String,
        pub shares: Uint128,
    }

    #[cw_serde]
    pub enum CMQueryMsg {
        /// Enumerate debt sha|res for all token positions; start_after accepts (account_id, denom)
        AllDebtShares {
            start_after: Option<(String, String)>,
            limit: Option<u32>,
        },
        /// Enumerate total debt shares for all supported coins; start_after accepts denom string
        AllTotalDebtShares {
            start_after: Option<String>,
            limit: Option<u32>,
        },
    }
}

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    // Lock red-bank to prevent any operations during migration.
    // Unlock is executed after full migration in `migrate_debts`.
    MIGRATION_GUARD.try_lock(deps.storage)?;

    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}

pub fn execute_migration(
    deps: DepsMut,
    info: MessageInfo,
    msg: MigrateV2ToV2_0_1,
) -> Result<Response, ContractError> {
    match msg {
        MigrateV2ToV2_0_1::Debts {
            limit,
        } => migrate_debts(deps, limit as usize),
        MigrateV2ToV2_0_1::CreditManagerDebts {
            limit,
        } => migrate_cm_debts(deps, limit as usize),
        MigrateV2ToV2_0_1::ClearV2State {} => {
            OWNER.assert_owner(deps.storage, &info.sender)?;
            clear_v2_state(deps)
        }
    }
}

fn migrate_debts(deps: DepsMut, limit: usize) -> Result<Response, ContractError> {
    // Only allow to migrate debts if guard is locked via `migrate` entrypoint
    MIGRATION_GUARD.assert_locked(deps.storage)?;

    // convert last key from v2 to v1
    let debts_last_key = DEBTS.last(deps.storage)?.map(|kv| kv.0);
    let debts_last_key = if let Some((user_id_key, denom)) = debts_last_key {
        let user_id: UserId = user_id_key.try_into()?;
        Some((user_id.addr, denom))
    } else {
        None
    };

    // last key from new debts is first key (excluded) for v1 debts during pagination
    let start_after =
        debts_last_key.as_ref().map(|(addr, denom)| Bound::exclusive((addr, denom.as_str())));
    let mut v2_debts = v2_state::DEBTS
        .range(deps.storage, start_after, None, Order::Ascending)
        .take(limit + 1)
        .collect::<StdResult<Vec<_>>>()?;

    let has_more = v2_debts.len() > limit;
    if has_more {
        v2_debts.pop(); // Remove the extra item used for checking if there are more items
    }

    // save debts
    for ((user_addr, denom), debt) in v2_debts.into_iter() {
        let user_id = UserId::credit_manager(user_addr, "".to_string());
        let user_id_key: UserIdKey = user_id.try_into()?;
        DEBTS.save(deps.storage, (&user_id_key, &denom), &debt)?;
    }

    Ok(Response::new()
        .add_attribute("action", "migrate_debts")
        .add_attribute(
            "result",
            if has_more {
                "in_progress"
            } else {
                "done"
            },
        )
        .add_attribute("start_after", key_to_str(debts_last_key))
        .add_attribute("limit", limit.to_string())
        .add_attribute("has_more", has_more.to_string()))
}

fn migrate_cm_debts(deps: DepsMut, limit: usize) -> Result<Response, ContractError> {
    // Only allow to migrate debts if guard is locked via `migrate` entrypoint
    MIGRATION_GUARD.assert_locked(deps.storage)?;

    // Get the credit manager address
    let config = CONFIG.load(deps.storage)?;
    let cm_address = deps
        .querier
        .query_wasm_smart::<AddressResponseItem>(
            config.address_provider.to_string(),
            &AddressProviderQueryMsg::Address(MarsAddressType::CreditManager),
        )?
        .address;

    let last_debt_key = DEBTS.last(deps.storage)?.map(|d| d.0);

    // Need to check if last key addr == cm address
    let last_debt_key = if let Some((user_id_key, denom)) = last_debt_key {
        let user_id: UserId = user_id_key.try_into()?;

        // On the first iteration the last_debt might not be the CM + accountId
        if !user_id.acc_id.is_empty() {
            Some((user_id.acc_id, denom))
        } else {
            None
        }
    } else {
        None
    };

    // Get all debt shares for the (account,denom) that has not been migrated yet
    let mut debts = deps.querier.query_wasm_smart::<Vec<v2_state::SharesResponseItem>>(
        &cm_address,
        &v2_state::CMQueryMsg::AllDebtShares {
            start_after: last_debt_key.clone(),
            limit: Some(limit as u32 + 1),
        },
    )?;

    let has_more = debts.len() > limit;
    if has_more {
        debts.pop(); // Remove the extra item used for checking if there are more items
    }

    // Get all total shares of the credit manager
    let all_total_debt_shares = deps.querier.query_wasm_smart::<Vec<v2_state::DebtShares>>(
        &cm_address,
        &v2_state::CMQueryMsg::AllTotalDebtShares {
            start_after: None,
            limit: Some(15), // There are only 12 borrowable assets currently, but set limit to 15 to be safe.
        },
    )?;

    let cm_user_id = UserId::credit_manager(Addr::unchecked(&cm_address), "".to_string());
    let cm_user_id_key: UserIdKey = cm_user_id.try_into()?;

    // For each debt, calculated the amount_scaled and save it in DEBTS
    for debt in debts.into_iter() {
        let total_cm_debt_amount_scaled = DEBTS
            .may_load(deps.storage, (&cm_user_id_key, &debt.denom))?
            .unwrap_or_default()
            .amount_scaled;

        let total_debt_shares = all_total_debt_shares
            .iter()
            .find(|debts_shares| debts_shares.denom == debt.denom)
            .unwrap()
            .shares;

        let amount_scaled =
            total_cm_debt_amount_scaled.checked_mul_ceil((debt.shares, total_debt_shares))?;

        let user_id = UserId::credit_manager(Addr::unchecked(&cm_address), debt.account_id);
        let user_id_key: UserIdKey = user_id.try_into()?;

        DEBTS.save(
            deps.storage,
            (&user_id_key, &debt.denom),
            &Debt {
                amount_scaled,
                uncollateralized: false,
            },
        )?;
    }

    // Migration of shares has finished
    if !has_more {
        // Remove the CM debt without account id
        DEBTS.prefix(&cm_user_id_key).clear(deps.storage, None);

        // red-bank locked via `migrate` entrypoint. Unlock red-bank after full migration
        MIGRATION_GUARD.try_unlock(deps.storage)?;
    }

    Ok(Response::new()
        .add_attribute("action", "migrate_cm_debts")
        .add_attribute(
            "result",
            if has_more {
                "in_progress"
            } else {
                "done"
            },
        )
        .add_attribute("start_after", key_str_to_str(last_debt_key))
        .add_attribute("limit", limit.to_string())
        .add_attribute("has_more", has_more.to_string()))
}

fn key_to_str(key: Option<(Addr, String)>) -> String {
    key.map(|(addr, denom)| format!("{}-{}", addr, denom)).unwrap_or("none".to_string())
}

fn key_str_to_str(key: Option<(String, String)>) -> String {
    key.map(|(addr, denom)| format!("{}-{}", addr, denom)).unwrap_or("none".to_string())
}

fn clear_v2_state(deps: DepsMut) -> Result<Response, ContractError> {
    // It is safe to clear v1 state only after full migration (guard is unlocked)
    MIGRATION_GUARD.assert_unlocked(deps.storage)?;
    v2_state::DEBTS.clear(deps.storage);
    Ok(Response::new().add_attribute("action", "clear_v2_state"))
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;

    use cosmwasm_std::{attr, testing::mock_dependencies, Addr, Uint128};
    use mars_utils::error::GuardError;

    use super::*;

    #[test]
    fn cannot_migrate_v2_debts_without_lock() {
        let mut deps = mock_dependencies();

        let res_error = migrate_debts(deps.as_mut(), 10).unwrap_err();
        assert_eq!(res_error, ContractError::Guard(GuardError::Inactive {}));
    }

    #[test]
    fn empty_v2_debts() {
        let mut deps = mock_dependencies();

        MIGRATION_GUARD.try_lock(deps.as_mut().storage).unwrap();

        let res = migrate_debts(deps.as_mut(), 10).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_debts"),
                attr("result", "done"),
                attr("start_after", "none"),
                attr("limit", "10"),
                attr("has_more", "false"),
            ]
        );
    }

    #[test]
    fn migrate_v2_debts() {
        let mut deps = mock_dependencies();

        MIGRATION_GUARD.try_lock(deps.as_mut().storage).unwrap();

        // prepare v1 debts
        let user_1_osmo_debt = Debt {
            amount_scaled: Uint128::new(12345),
            uncollateralized: false,
        };
        v2_state::DEBTS
            .save(deps.as_mut().storage, (&Addr::unchecked("user_1"), "uosmo"), &user_1_osmo_debt)
            .unwrap();
        let user_2_atom_debt = Debt {
            amount_scaled: Uint128::new(1),
            uncollateralized: false,
        };
        v2_state::DEBTS
            .save(deps.as_mut().storage, (&Addr::unchecked("user_2"), "uatom"), &user_2_atom_debt)
            .unwrap();
        let user_2_jake_debt = Debt {
            amount_scaled: Uint128::new(1023),
            uncollateralized: false,
        };
        v2_state::DEBTS
            .save(deps.as_mut().storage, (&Addr::unchecked("user_2"), "ujake"), &user_2_jake_debt)
            .unwrap();
        let user_3_jake_debt = Debt {
            amount_scaled: Uint128::new(1111111),
            uncollateralized: false,
        };
        v2_state::DEBTS
            .save(deps.as_mut().storage, (&Addr::unchecked("user_3"), "ujake"), &user_3_jake_debt)
            .unwrap();
        let user_1_axl_debt = Debt {
            amount_scaled: Uint128::new(123456789),
            uncollateralized: false,
        };
        v2_state::DEBTS
            .save(deps.as_mut().storage, (&Addr::unchecked("user_1"), "uaxl"), &user_1_axl_debt)
            .unwrap();

        // migrate first 2 debts
        let res = migrate_debts(deps.as_mut(), 2).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_debts"),
                attr("result", "in_progress"),
                attr("start_after", "none"),
                attr("limit", "2"),
                attr("has_more", "true"),
            ]
        );

        // in new debts we should have 2 debts
        let debts = DEBTS
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<HashMap<_, _>>>()
            .unwrap();
        assert_eq!(debts.len(), 2);

        // migrate next 2 debts
        let res = migrate_debts(deps.as_mut(), 2).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_debts"),
                attr("result", "in_progress"),
                attr("start_after", "user_1-uosmo"),
                attr("limit", "2"),
                attr("has_more", "true"),
            ]
        );

        // in new debts we should have more 2 debts, 4 in total
        let debts = DEBTS
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<HashMap<_, _>>>()
            .unwrap();
        assert_eq!(debts.len(), 4);

        // migrate next 2 debts, we have only 1 left
        let res = migrate_debts(deps.as_mut(), 2).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_debts"),
                attr("result", "done"),
                attr("start_after", "user_2-ujake"),
                attr("limit", "2"),
                attr("has_more", "false"),
            ]
        );

        // in new debts we should have more 1 debt, 5 in total
        let debts = DEBTS
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<StdResult<HashMap<_, _>>>()
            .unwrap();
        assert_eq!(debts.len(), 5);

        // compare values
        let user_id = UserId::credit_manager(Addr::unchecked("user_1"), "".to_string());
        let user_1_id_key: UserIdKey = user_id.try_into().unwrap();
        assert_eq!(
            debts.get(&(user_1_id_key.clone(), "uosmo".to_string())).unwrap(),
            &user_1_osmo_debt
        );
        assert_eq!(debts.get(&(user_1_id_key, "uaxl".to_string())).unwrap(), &user_1_axl_debt);
        let user_id = UserId::credit_manager(Addr::unchecked("user_2"), "".to_string());
        let user_2_id_key: UserIdKey = user_id.try_into().unwrap();
        assert_eq!(
            debts.get(&(user_2_id_key.clone(), "uatom".to_string())).unwrap(),
            &user_2_atom_debt
        );
        assert_eq!(debts.get(&(user_2_id_key, "ujake".to_string())).unwrap(), &user_2_jake_debt);
        let user_id = UserId::credit_manager(Addr::unchecked("user_3"), "".to_string());
        let user_3_id_key: UserIdKey = user_id.try_into().unwrap();
        assert_eq!(debts.get(&(user_3_id_key, "ujake".to_string())).unwrap(), &user_3_jake_debt);

        // Try migrating once more, see that result is done
        let res = migrate_debts(deps.as_mut(), 2).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "migrate_debts"),
                attr("result", "done"),
                attr("start_after", "user_3-ujake"),
                attr("limit", "2"),
                attr("has_more", "false"),
            ]
        );
    }

    // todo: Implement migrate_cm_debts tests
}
