use std::cmp::{max, min};

use cosmwasm_std::{
    Addr, BlockInfo, Decimal, Deps, MessageInfo, Order, OverflowError, OverflowOperation,
    QuerierWrapper, StdError, StdResult, Storage, Uint128,
};
use cw_storage_plus::Bound;
use mars_red_bank_types::{
    address_provider::{self, MarsAddressType},
    incentives::IncentiveState,
    red_bank,
};

use crate::{
    state::{
        EMISSIONS, EPOCH_DURATION, INCENTIVE_STATES, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS,
        WHITELIST,
    },
    ContractError,
};

/// A helper enum to represent a storage that can either be immutable or mutable. This is useful
/// to create functions that should mutate state on Execute but not on Query.
pub enum MaybeMutStorage<'a> {
    Immutable(&'a dyn Storage),
    Mutable(&'a mut dyn Storage),
}

impl<'a> From<&'a dyn Storage> for MaybeMutStorage<'a> {
    fn from(storage: &'a dyn Storage) -> Self {
        MaybeMutStorage::Immutable(storage)
    }
}

impl<'a> From<&'a mut dyn Storage> for MaybeMutStorage<'a> {
    fn from(storage: &'a mut dyn Storage) -> Self {
        MaybeMutStorage::Mutable(storage)
    }
}

impl MaybeMutStorage<'_> {
    pub fn to_storage(&self) -> &dyn Storage {
        match self {
            MaybeMutStorage::Immutable(storage) => *storage,
            MaybeMutStorage::Mutable(storage) => *storage,
        }
    }
}

/// Validates that a incentive schedule to be added is valid. This checks that:
/// - start_time is in the future
/// - duration is a multiple of epoch duration
/// - enough tokens are sent to cover the entire duration
/// - start_time is a multiple of epoch duration away from any other existing incentive
///  for the same collateral denom and incentive denom tuple
pub fn validate_incentive_schedule(
    storage: &dyn Storage,
    info: &MessageInfo,
    epoch_duration: u64,
    current_time: u64,
    collateral_denom: &str,
    incentive_denom: &str,
    emission_per_second: Uint128,
    start_time: u64,
    duration: u64,
) -> Result<(), ContractError> {
    // start_time can't be less that current block time
    if start_time < current_time {
        return Err(ContractError::InvalidIncentive {
            reason: "start_time can't be less than current block time".to_string(),
        });
    }
    if duration == 0 {
        return Err(ContractError::InvalidIncentive {
            reason: "duration can't be zero".to_string(),
        });
    }
    // Duration must be a multiple of epoch duration
    if duration % epoch_duration != 0 {
        return Err(ContractError::InvalidDuration {
            epoch_duration,
        });
    }
    // Emission must meet minimum amount
    let min_emission = WHITELIST.load(storage, incentive_denom)?;
    if emission_per_second < min_emission {
        return Err(ContractError::InvalidIncentive {
            reason: format!(
                "emission_per_second must be greater than min_emission: {}",
                min_emission
            ),
        });
    }
    // Enough tokens must be sent to cover the entire duration
    let total_emission = emission_per_second * Uint128::from(duration);
    if info.funds.len() != 1 || info.funds[0].amount != total_emission {
        return Err(ContractError::InvalidFunds {
            expected: total_emission,
        });
    }
    // Start time must be a multiple of epoch duration away from any other existing incentive
    // for the same collateral denom and incentive denom tuple. We do this so we have exactly one
    // incentive schedule per epoch, to limit gas usage.
    let old_schedule = EMISSIONS
        .prefix((collateral_denom, incentive_denom))
        .range(storage, None, None, Order::Ascending)
        .next()
        .transpose()?;
    if let Some((existing_start_time, _)) = old_schedule {
        let start_time_diff = start_time.abs_diff(existing_start_time);
        if start_time_diff % epoch_duration != 0 {
            return Err(ContractError::InvalidStartTime {
                epoch_duration,
                existing_start_time,
            });
        }
    }

    Ok(())
}

/// Queries the total scaled collateral for a given collateral denom from the red bank contract
pub fn query_red_bank_total_collateral(
    deps: Deps,
    address_provider: &Addr,
    collateral_denom: &str,
) -> StdResult<Uint128> {
    let red_bank_addr = address_provider::helpers::query_contract_addr(
        deps,
        address_provider,
        MarsAddressType::RedBank,
    )?;
    let market: red_bank::Market = deps.querier.query_wasm_smart(
        red_bank_addr,
        &red_bank::QueryMsg::Market {
            denom: collateral_denom.to_string(),
        },
    )?;
    Ok(market.collateral_total_scaled)
}

/// Updates the incentive index for a collateral denom and incentive denom tuple. This function
/// should be called every time a user's collateral balance changes, when a new incentive schedule
/// is added, or when a user claims rewards.
pub fn update_incentive_index(
    storage: &mut MaybeMutStorage,
    collateral_denom: &str,
    incentive_denom: &str,
    total_collateral: Uint128,
    current_block_time: u64,
) -> StdResult<IncentiveState> {
    let epoch_duration = EPOCH_DURATION.load(storage.to_storage())?;

    let mut incentive_state = INCENTIVE_STATES
        .may_load(storage.to_storage(), (collateral_denom, incentive_denom))?
        .unwrap_or_else(|| IncentiveState {
            index: Decimal::zero(),
            last_updated: current_block_time,
        });

    // If incentive state is already up to date or there is no collateral, no need to update
    if incentive_state.last_updated == current_block_time || total_collateral.is_zero() {
        return Ok(incentive_state);
    }

    // Range over the emissions for all relevant epochs (those which have a start time before the
    // current block time)
    let emissions = EMISSIONS
        .prefix((collateral_denom, incentive_denom))
        .range(
            storage.to_storage(),
            None,
            Some(Bound::exclusive(current_block_time)),
            cosmwasm_std::Order::Ascending,
        )
        .collect::<StdResult<Vec<_>>>()?;

    for (start_time, emission_per_second) in emissions {
        let end_time_sec = start_time + epoch_duration;
        let time_start = max(start_time, incentive_state.last_updated);
        let time_end = min(current_block_time, end_time_sec);
        incentive_state.index = compute_incentive_index(
            incentive_state.index,
            emission_per_second,
            total_collateral,
            time_start,
            time_end,
        )?;

        // If incentive schedule is over, remove it from storage
        if let MaybeMutStorage::Mutable(storage) = storage {
            if end_time_sec <= current_block_time {
                EMISSIONS.remove(*storage, (collateral_denom, incentive_denom, start_time));
            }
        }
    }

    // Set last updated time
    incentive_state.last_updated = current_block_time;

    // Save updated index if storage is mutable
    if let MaybeMutStorage::Mutable(storage) = storage {
        INCENTIVE_STATES.save(*storage, (collateral_denom, incentive_denom), &incentive_state)?;
    }

    Ok(incentive_state)
}

/// Computes the new incentive index for a given collateral denom and incentive denom tuple
pub fn compute_incentive_index(
    previous_index: Decimal,
    emission_per_second: Uint128,
    total_amount_scaled: Uint128,
    time_start: u64,
    time_end: u64,
) -> StdResult<Decimal> {
    if time_start > time_end {
        return Err(StdError::overflow(OverflowError::new(
            OverflowOperation::Sub,
            time_start,
            time_end,
        )));
    }
    let seconds_elapsed = time_end - time_start;
    let emission_for_elapsed_seconds =
        emission_per_second.checked_mul(Uint128::from(seconds_elapsed))?;
    let new_index =
        previous_index + Decimal::from_ratio(emission_for_elapsed_seconds, total_amount_scaled);
    Ok(new_index)
}

/// Computes user accrued rewards using the difference between incentive index and
/// user current index.
/// incentive index should be up to date.
pub fn compute_user_accrued_rewards(
    user_amount_scaled: Uint128,
    user_asset_index: Decimal,
    asset_incentive_index: Decimal,
) -> StdResult<Uint128> {
    let result = (user_amount_scaled * asset_incentive_index)
        .checked_sub(user_amount_scaled * user_asset_index)?;
    Ok(result)
}

/// Computes unclaimed rewards for a given user. Also updates the user's index to the current
/// incentive index if storage is mutable.
/// NB: Does not store the updated unclaimed rewards in storage.
pub fn compute_user_unclaimed_rewards(
    storage: &mut MaybeMutStorage,
    querier: &QuerierWrapper,
    block: &BlockInfo,
    red_bank_addr: &Addr,
    user_addr: &Addr,
    collateral_denom: &str,
    incentive_denom: &str,
) -> StdResult<Uint128> {
    let mut unclaimed_rewards = USER_UNCLAIMED_REWARDS
        .may_load(storage.to_storage(), (user_addr, collateral_denom, incentive_denom))?
        .unwrap_or_else(Uint128::zero);

    // Get asset user balances and total supply
    let collateral: red_bank::UserCollateralResponse = querier.query_wasm_smart(
        red_bank_addr,
        &red_bank::QueryMsg::UserCollateral {
            user: user_addr.to_string(),
            denom: collateral_denom.to_string(),
        },
    )?;
    let market: red_bank::Market = querier.query_wasm_smart(
        red_bank_addr,
        &red_bank::QueryMsg::Market {
            denom: collateral_denom.to_string(),
        },
    )?;

    // If user's balance is 0 there should be no rewards to accrue, so we don't care about
    // updating indexes. If the user's balance changes, the indexes will be updated correctly at
    // that point in time.
    if collateral.amount_scaled.is_zero() {
        return Ok(unclaimed_rewards);
    }

    let incentive_state = update_incentive_index(
        storage,
        collateral_denom,
        incentive_denom,
        market.collateral_total_scaled,
        block.time.seconds(),
    )?;

    let user_asset_index = USER_ASSET_INDICES
        .may_load(storage.to_storage(), (user_addr, collateral_denom, incentive_denom))?
        .unwrap_or_else(Decimal::zero);

    if user_asset_index != incentive_state.index {
        // Compute user accrued rewards and update user index
        let asset_accrued_rewards = compute_user_accrued_rewards(
            collateral.amount_scaled,
            user_asset_index,
            incentive_state.index,
        )?;
        unclaimed_rewards += asset_accrued_rewards;
    }

    // If state is mutable, commit updated user index
    if let MaybeMutStorage::Mutable(storage) = storage {
        if user_asset_index != incentive_state.index {
            USER_ASSET_INDICES.save(
                *storage,
                (user_addr, collateral_denom, incentive_denom),
                &incentive_state.index,
            )?
        }
    }

    Ok(unclaimed_rewards)
}
