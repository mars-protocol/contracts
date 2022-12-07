use cosmwasm_std::{
    Addr, BlockInfo, Decimal, Deps, Order, OverflowError, OverflowOperation, StdError, StdResult,
    Uint128,
};
use std::cmp::{max, min};

use mars_outpost::incentives::AssetIncentive;
use mars_outpost::red_bank;

use crate::state::{ASSET_INCENTIVES, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS};

/// Updates asset incentive index and last updated timestamp by computing
/// how many rewards were accrued since last time updated given incentive's
/// emission per second.
/// Total supply is the total (liquidity) token supply during the period being computed.
/// Note that this method does not commit updates to state as that should be executed by the
/// caller
pub fn asset_incentive_update_index(
    asset_incentive: &mut AssetIncentive,
    total_amount_scaled: Uint128,
    current_block_time: u64,
) -> StdResult<()> {
    let end_time_sec = asset_incentive.start_time.plus_seconds(asset_incentive.duration).seconds();
    if (current_block_time != asset_incentive.last_updated)
        && current_block_time >= asset_incentive.start_time.seconds()
        && asset_incentive.last_updated < end_time_sec
        && !total_amount_scaled.is_zero()
        && !asset_incentive.emission_per_second.is_zero()
    {
        let start_time_sec = asset_incentive.start_time.seconds();
        let time_start = max(start_time_sec, asset_incentive.last_updated);
        let time_end = min(current_block_time, end_time_sec);
        asset_incentive.index = asset_incentive_compute_index(
            asset_incentive.index,
            asset_incentive.emission_per_second,
            total_amount_scaled,
            time_start,
            time_end,
        )?;
    }
    asset_incentive.last_updated = current_block_time;
    Ok(())
}

pub fn asset_incentive_compute_index(
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

/// Computes user accrued rewards using the difference between asset_incentive index and
/// user current index
/// asset_incentives index should be up to date.
pub fn user_compute_accrued_rewards(
    user_amount_scaled: Uint128,
    user_asset_index: Decimal,
    asset_incentive_index: Decimal,
) -> StdResult<Uint128> {
    let result = (user_amount_scaled * asset_incentive_index)
        .checked_sub(user_amount_scaled * user_asset_index)?;
    Ok(result)
}

/// Result of querying and updating the status of the user and a give asset incentives in order to
/// compute unclaimed rewards.
pub struct UserAssetIncentiveStatus {
    /// Denom of the asset that's the incentives target
    pub denom: String,
    /// Current user index's value on the contract store (not updated by current asset index)
    pub user_index_current: Decimal,
    /// Asset incentive with values updated to the current block (not neccesarily commited
    /// to storage)
    pub asset_incentive_updated: AssetIncentive,
}

pub fn compute_user_unclaimed_rewards(
    deps: Deps,
    block: &BlockInfo,
    red_bank_addr: &Addr,
    user_addr: &Addr,
) -> StdResult<(Uint128, Vec<UserAssetIncentiveStatus>)> {
    let mut total_unclaimed_rewards =
        USER_UNCLAIMED_REWARDS.may_load(deps.storage, user_addr)?.unwrap_or_else(Uint128::zero);

    let result_asset_incentives: StdResult<Vec<_>> =
        ASSET_INCENTIVES.range(deps.storage, None, None, Order::Ascending).collect();

    let mut user_asset_incentive_statuses_to_update: Vec<UserAssetIncentiveStatus> = vec![];

    for (denom, mut asset_incentive) in result_asset_incentives? {
        // Get asset user balances and total supply
        let collateral: red_bank::UserCollateralResponse = deps.querier.query_wasm_smart(
            red_bank_addr,
            &red_bank::QueryMsg::UserCollateral {
                user: user_addr.to_string(),
                denom: denom.clone(),
            },
        )?;
        let market: red_bank::Market = deps.querier.query_wasm_smart(
            red_bank_addr,
            &red_bank::QueryMsg::Market {
                denom: denom.clone(),
            },
        )?;

        // If user's balance is 0 there should be no rewards to accrue, so we don't care about
        // updating indexes. If the user's balance changes, the indexes will be updated correctly at
        // that point in time.
        if collateral.amount_scaled.is_zero() {
            continue;
        }

        asset_incentive_update_index(
            &mut asset_incentive,
            market.collateral_total_scaled,
            block.time.seconds(),
        )?;

        let user_asset_index = USER_ASSET_INDICES
            .may_load(deps.storage, (user_addr, &denom))?
            .unwrap_or_else(Decimal::zero);

        if user_asset_index != asset_incentive.index {
            // Compute user accrued rewards and update user index
            let asset_accrued_rewards = user_compute_accrued_rewards(
                collateral.amount_scaled,
                user_asset_index,
                asset_incentive.index,
            )?;
            total_unclaimed_rewards += asset_accrued_rewards;
        }

        user_asset_incentive_statuses_to_update.push(UserAssetIncentiveStatus {
            denom,
            user_index_current: user_asset_index,
            asset_incentive_updated: asset_incentive,
        });
    }

    Ok((total_unclaimed_rewards, user_asset_incentive_statuses_to_update))
}
