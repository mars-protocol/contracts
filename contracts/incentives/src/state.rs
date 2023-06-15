use cosmwasm_std::{Addr, Decimal, Order, StdResult, Storage, Uint128};
use cw_item_set::Set;
use cw_storage_plus::{Bound, Item, Map, PrefixBound};
use mars_owner::Owner;
use mars_red_bank_types::incentives::{Config, IncentiveSchedule, IncentiveState};

use crate::ContractError;

/// The owner of the contract
pub const OWNER: Owner = Owner::new("owner");

/// The configuration of the contract
pub const CONFIG: Item<Config> = Item::new("config");

/// A set containing all whitelisted incentive denoms. Incentives can only be added for denoms in
/// this set.
pub const WHITELIST: Set<&str> = Set::new("whitelist", "whitelist_counter");

/// A map containing the incentive index and last updated time for a given collateral and incentive
/// denom. The key is (collateral denom, incentive denom).
pub const INCENTIVE_STATES: Map<(&str, &str), IncentiveState> = Map::new("incentive_states");

/// A map containing incentive schedules for a given collateral and incentive denom. The key is
/// (collateral denom, incentive denom, schedule start time).
pub const INCENTIVE_SCHEDULES: Map<(&str, &str, u64), IncentiveSchedule> =
    Map::new("incentive_schedules");

/// A map containing the incentive index for a given user, collateral denom and incentive denom.
/// The key is (user address, collateral denom, incentive denom).
pub const USER_ASSET_INDICES: Map<(&Addr, &str, &str), Decimal> = Map::new("indices");

/// A map containing the amount of unclaimed incentives for a given user and incentive denom.
/// The key is (user address, collateral denom, incentive denom).
pub const USER_UNCLAIMED_REWARDS: Map<(&Addr, &str, &str), Uint128> = Map::new("unclaimed_rewards");

/// The default limit for pagination over asset incentives
pub const DEFAULT_LIMIT: u32 = 5;

/// The maximum limit for pagination over asset incentives
/// TODO: Remove MAX_LIMIT? What is the purpose? Surely better to have the limit be whatever is the max gas limit?
pub const MAX_LIMIT: u32 = 10;

/// Helper function to update unclaimed rewards for a given user, collateral denom and incentive
/// denom. Adds `accrued_rewards` to the existing amount.
pub fn increase_unclaimed_rewards(
    storage: &mut dyn Storage,
    user_addr: &Addr,
    collateral_denom: &str,
    incentive_denom: &str,
    accrued_rewards: Uint128,
) -> StdResult<()> {
    USER_UNCLAIMED_REWARDS.update(
        storage,
        (user_addr, collateral_denom, incentive_denom),
        |ur: Option<Uint128>| -> StdResult<Uint128> {
            Ok(ur.map_or_else(|| accrued_rewards, |r| r + accrued_rewards))
        },
    )?;
    Ok(())
}

/// Returns asset incentives, with optional pagination.
/// Caller should make sure that if start_after_incentive_denom is supplied, then
/// start_after_collateral_denom is also supplied.
pub fn paginate_incentive_states(
    storage: &dyn Storage,
    start_after_collateral_denom: Option<String>,
    start_after_incentive_denom: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<((String, String), IncentiveState)>, ContractError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    Ok(match (start_after_collateral_denom.as_ref(), start_after_incentive_denom.as_ref()) {
        (Some(collat_denom), Some(incen_denom)) => {
            let start = Bound::exclusive((collat_denom.as_str(), incen_denom.as_str()));
            INCENTIVE_STATES.range(storage, Some(start), None, Order::Ascending)
        }
        (Some(collat_denom), None) => {
            let start = PrefixBound::exclusive(collat_denom.as_str());
            INCENTIVE_STATES.prefix_range(storage, Some(start), None, Order::Ascending)
        }
        (None, Some(_)) => return Err(ContractError::InvalidPaginationParams),
        _ => INCENTIVE_STATES.range(storage, None, None, Order::Ascending),
    }
    .take(limit)
    .collect::<StdResult<Vec<_>>>()?)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::MockStorage;

    use super::*;

    #[test]
    fn paginate_incentive_states_works() {
        let mut storage = MockStorage::new();

        //store some incentives
        let asset_incentive = IncentiveState {
            index: Decimal::zero(),
            last_updated: 0,
        };
        let incentives = vec![
            (("collat1".to_string(), "incen1".to_string()), asset_incentive.clone()),
            (("collat1".to_string(), "incen2".to_string()), asset_incentive.clone()),
            (("collat2".to_string(), "incen1".to_string()), asset_incentive.clone()),
            (("collat2".to_string(), "incen2".to_string()), asset_incentive.clone()),
        ];
        for ((collat, incen), incentive) in incentives.iter() {
            INCENTIVE_STATES
                .save(&mut storage, (collat.as_str(), incen.as_str()), &incentive)
                .unwrap();
        }

        // No pagination
        let res = paginate_incentive_states(&storage, None, None, None).unwrap();
        assert_eq!(res, incentives);

        // Start after collateral denom
        let res =
            paginate_incentive_states(&storage, Some("collat1".to_string()), None, None).unwrap();
        println!("start after collat1: {:?}", res);
        println!("expected: {:?}", incentives[2..].to_vec());
        assert_eq!(res, incentives[2..]);

        // Start after collateral denom and incentive denom
        let res = paginate_incentive_states(
            &storage,
            Some("collat1".to_string()),
            Some("incen1".to_string()),
            None,
        )
        .unwrap();
        assert_eq!(res, incentives[1..]);
        let res = paginate_incentive_states(
            &storage,
            Some("collat1".to_string()),
            Some("incen2".to_string()),
            None,
        )
        .unwrap();
        assert_eq!(res, incentives[2..]);

        // Limit
        let res = paginate_incentive_states(&storage, None, None, Some(2)).unwrap();
        assert_eq!(res, incentives[..2].to_vec());
    }
}
