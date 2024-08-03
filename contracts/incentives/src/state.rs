use cosmwasm_std::{Addr, Decimal, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Bound, Item, Map, PrefixBound};
use mars_owner::Owner;
use mars_types::{
    incentives::{Config, IncentiveState},
    keys::{UserId, UserIdKey},
};
use mars_utils::guard::Guard;

use crate::ContractError;

/// The owner of the contract
pub const OWNER: Owner = Owner::new("owner");

/// The configuration of the contract
pub const CONFIG: Item<Config> = Item::new("config");

/// The amount of time in seconds for each incentive epoch. This is the minimum amount of time
/// that an incentive can last, and each incentive must be a multiple of this duration.
pub const EPOCH_DURATION: Item<u64> = Item::new("epoch_duration");

/// A set containing all whitelisted incentive denoms as well as the minimum emission amount for each.
/// Incentives can only be added for denoms in this set.
pub const WHITELIST: Map<&str, Uint128> = Map::new("whitelist");

/// A counter for the number of whitelisted incentive denoms. This is used to enforce a maximum
/// number of whitelisted denoms.
pub const WHITELIST_COUNT: Item<u8> = Item::new("whitelist_count");

/// A map containing the incentive index and last updated time for a given collateral and incentive
/// denom. The key is (collateral denom, incentive denom).
pub const INCENTIVE_STATES: Map<(&str, &str), IncentiveState> = Map::new("incentive_states");

/// A map containing the global incentive index for a given lp token
/// The key is (lp token denom, incentive denom).
pub const ASTRO_INCENTIVE_STATES: Map<(&str, &str), Decimal> =
    Map::new("astroport_incentive_states");

/// A map containing the individual incentive index for each unique user
/// Note - this may contain many denoms for one user
/// The key is (account_id, lp_token_denom, reward_denom)
pub const USER_ASTRO_INCENTIVE_STATES: Map<(&str, &str, &str), Decimal> =
    Map::new("user_astroport_incentive_states");

/// A map containing emission speeds (incentive tokens per second) for a given collateral and
/// incentive denom. The key is (collateral denom, incentive denom, schedule start time).
pub const EMISSIONS: Map<(&str, &str, u64), Uint128> = Map::new("emissions");

/// A map containing the incentive index for a given user, collateral denom and incentive denom.
/// The key is (user address with optional account id, collateral denom, incentive denom).
pub const USER_ASSET_INDICES: Map<(&UserIdKey, &str, &str), Decimal> = Map::new("indices_v2");

/// A map containing the amount of unclaimed incentives for a given user and incentive denom.
/// The key is (user address with optional account id, collateral denom, incentive denom).
pub const USER_UNCLAIMED_REWARDS: Map<(&UserIdKey, &str, &str), Uint128> =
    Map::new("unclaimed_rewards_v2");

/// Used to mark the contract as locked during migrations
pub const MIGRATION_GUARD: Guard = Guard::new("guard");

/// The default limit for pagination
pub const DEFAULT_LIMIT: u32 = 5;

/// The maximum limit for pagination
pub const MAX_LIMIT: u32 = 10;

/// User LP positions staked in the astroport incentives contract. Returns amount
/// The key is (account_id, lp_denom)
pub const ASTRO_USER_LP_DEPOSITS: Map<(&str, &str), Uint128> = Map::new("lp_deposits");

/// Total LP deposits in the astroport incentives contract. Returns amount
/// The key is lp_denom
pub const ASTRO_TOTAL_LP_DEPOSITS: Map<&str, Uint128> = Map::new("total_lp_deposits");

/// Helper function to update unclaimed rewards for a given user, collateral denom and incentive
/// denom. Adds `accrued_rewards` to the existing amount.
pub fn increase_unclaimed_rewards(
    storage: &mut dyn Storage,
    user_addr: &Addr,
    acc_id: &str,
    collateral_denom: &str,
    incentive_denom: &str,
    accrued_rewards: Uint128,
) -> StdResult<()> {
    let user_id = UserId::credit_manager(user_addr.clone(), acc_id.to_string());
    let user_id_key: UserIdKey = user_id.try_into()?;

    USER_UNCLAIMED_REWARDS.update(
        storage,
        (&user_id_key, collateral_denom, incentive_denom),
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
            (("collat2".to_string(), "incen2".to_string()), asset_incentive),
        ];
        for ((collat, incen), incentive) in incentives.iter() {
            INCENTIVE_STATES
                .save(&mut storage, (collat.as_str(), incen.as_str()), incentive)
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
