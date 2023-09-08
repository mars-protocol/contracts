use std::collections::HashMap;

use cosmwasm_std::{DepsMut, Env, Order, Response, StdResult, Uint128};
use cw2::{assert_contract_version, set_contract_version};
use mars_owner::OwnerInit;
use mars_red_bank_types::incentives::{Config, IncentiveState, V2Updates};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION, MIN_EPOCH_DURATION},
    error::ContractError,
    state::{
        CONFIG, EPOCH_DURATION, INCENTIVE_STATES, OWNER, USER_ASSET_INDICES,
        USER_UNCLAIMED_REWARDS, WHITELIST, WHITELIST_COUNT,
    },
};

const FROM_VERSION: &str = "1.0.0";

pub mod v1_state {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Decimal, Uint128};
    use cw_storage_plus::{Item, Map};
    use mars_red_bank_types_old::incentives::{AssetIncentive, Config};

    pub const OWNER: Item<OwnerState> = Item::new("owner");
    pub const CONFIG: Item<Config> = Item::new("config");

    pub const ASSET_INCENTIVES: Map<&str, AssetIncentive> = Map::new("incentives");
    pub const USER_ASSET_INDICES: Map<(&Addr, &str), Decimal> = Map::new("indices");
    pub const USER_UNCLAIMED_REWARDS: Map<&Addr, Uint128> = Map::new("unclaimed_rewards");

    #[cw_serde]
    pub enum OwnerState {
        B(OwnerSetNoneProposed),
    }

    #[cw_serde]
    pub struct OwnerSetNoneProposed {
        pub owner: Addr,
    }

    pub fn current_owner(state: OwnerState) -> Addr {
        match state {
            OwnerState::B(b) => b.owner,
        }
    }

    // Copy of helpers from v1.0.0 tag:
    // https://github.com/mars-protocol/red-bank/blob/v1.0.0/contracts/incentives/src/helpers.rs
    // Included as dependency coudn't generate proper schema for mars-incentive, even with specified
    // version.
    pub mod helpers {
        use std::cmp::{max, min};

        use cosmwasm_std::{
            Decimal, OverflowError, OverflowOperation, StdError, StdResult, Uint128,
        };
        use mars_red_bank_types_old::incentives::AssetIncentive;

        /// Updates asset incentive index and last updated timestamp by computing
        /// how many rewards were accrued since last time updated given incentive's
        /// emission per second.
        /// Total supply is the total (liquidity) token supply during the period being computed.
        /// Note that this method does not commit updates to state as that should be executed by the
        /// caller
        pub fn update_asset_incentive_index(
            asset_incentive: &mut AssetIncentive,
            total_amount_scaled: Uint128,
            current_block_time: u64,
        ) -> StdResult<()> {
            let end_time_sec = asset_incentive.start_time + asset_incentive.duration;
            if (current_block_time != asset_incentive.last_updated)
                && current_block_time > asset_incentive.start_time
                && asset_incentive.last_updated < end_time_sec
                && !total_amount_scaled.is_zero()
                && !asset_incentive.emission_per_second.is_zero()
            {
                let time_start = max(asset_incentive.start_time, asset_incentive.last_updated);
                let time_end = min(current_block_time, end_time_sec);
                asset_incentive.index = compute_asset_incentive_index(
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

        pub fn compute_asset_incentive_index(
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
            let new_index = previous_index
                + Decimal::from_ratio(emission_for_elapsed_seconds, total_amount_scaled);
            Ok(new_index)
        }

        /// Computes user accrued rewards using the difference between asset_incentive index and
        /// user current index
        /// asset_incentives index should be up to date.
        pub fn compute_user_accrued_rewards(
            user_amount_scaled: Uint128,
            user_asset_index: Decimal,
            asset_incentive_index: Decimal,
        ) -> StdResult<Uint128> {
            let result = (user_amount_scaled * asset_incentive_index)
                .checked_sub(user_amount_scaled * user_asset_index)?;
            Ok(result)
        }
    }
}

pub fn migrate(mut deps: DepsMut, env: Env, updates: V2Updates) -> Result<Response, ContractError> {
    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    // Owner package updated, re-initializing
    let old_owner_state = v1_state::OWNER.load(deps.storage)?;
    let old_owner = v1_state::current_owner(old_owner_state);
    v1_state::OWNER.remove(deps.storage);
    OWNER.initialize(
        deps.storage,
        deps.api,
        OwnerInit::SetInitialOwner {
            owner: old_owner.to_string(),
        },
    )?;

    // CONFIG updated, re-initializing
    let old_config_state = v1_state::CONFIG.load(deps.storage)?;
    v1_state::CONFIG.remove(deps.storage);
    CONFIG.save(
        deps.storage,
        &Config {
            address_provider: old_config_state.address_provider,
            max_whitelisted_denoms: updates.max_whitelisted_denoms,
        },
    )?;

    // WHITELIST not existent in v1, initializing
    WHITELIST.save(deps.storage, &old_config_state.mars_denom, &Uint128::one())?;
    WHITELIST_COUNT.save(deps.storage, &1)?;

    // EPOCH_DURATION not existent in v1, initializing
    if updates.epoch_duration < MIN_EPOCH_DURATION {
        return Err(ContractError::EpochDurationTooShort {
            min_epoch_duration: MIN_EPOCH_DURATION,
        });
    }
    EPOCH_DURATION.save(deps.storage, &updates.epoch_duration)?;

    migrate_indices_and_unclaimed_rewards(&mut deps, env, &old_config_state.mars_denom)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}

// Migrate indices and unclaimed rewards from v1 to v2 with helpers from v1.0.0 tag:
// https://github.com/mars-protocol/red-bank/blob/v1.0.0/contracts/incentives/src/helpers.rs
//
// This is done by querying the Red Bank contract for the collateral total supply and
// user collateral amount for each collateral denom.
fn migrate_indices_and_unclaimed_rewards(
    deps: &mut DepsMut,
    env: Env,
    mars_denom: &str,
) -> Result<(), ContractError> {
    let current_block_time = env.block.time.seconds();

    let config = CONFIG.load(deps.storage)?;

    let red_bank_addr = mars_red_bank_types::address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        &config.address_provider,
        mars_red_bank_types::address_provider::MarsAddressType::RedBank,
    )?;

    let mut asset_incentives = v1_state::ASSET_INCENTIVES
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<HashMap<_, _>>>()?;
    v1_state::ASSET_INCENTIVES.clear(deps.storage);

    for (denom, asset_incentive) in asset_incentives.iter_mut() {
        let market: mars_red_bank_types::red_bank::Market = deps.querier.query_wasm_smart(
            red_bank_addr.clone(),
            &mars_red_bank_types::red_bank::QueryMsg::Market {
                denom: denom.clone(),
            },
        )?;

        v1_state::helpers::update_asset_incentive_index(
            asset_incentive,
            market.collateral_total_scaled,
            current_block_time,
        )?;

        // Update incentive state for collateral and incentive denom (Mars)
        INCENTIVE_STATES.save(
            deps.storage,
            (denom, mars_denom),
            &IncentiveState {
                index: asset_incentive.index,
                last_updated: current_block_time,
            },
        )?;
    }

    let user_asset_indices = v1_state::USER_ASSET_INDICES
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    v1_state::USER_ASSET_INDICES.clear(deps.storage);

    let mut user_unclaimed_rewards = v1_state::USER_UNCLAIMED_REWARDS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<HashMap<_, _>>>()?;
    v1_state::USER_UNCLAIMED_REWARDS.clear(deps.storage);

    for ((user, denom), user_asset_index) in user_asset_indices {
        let collateral: mars_red_bank_types::red_bank::UserCollateralResponse =
            deps.querier.query_wasm_smart(
                red_bank_addr.clone(),
                &mars_red_bank_types::red_bank::QueryMsg::UserCollateral {
                    user: user.to_string(),
                    account_id: None,
                    denom: denom.clone(),
                },
            )?;

        // Get asset incentive for a denom. It should be available but just in case we don't unwrap
        let denom_idx = asset_incentives.get(&denom);
        if let Some(asset_incentive) = denom_idx {
            // Since we didn't track unclaimed rewards per collateral denom in v1 we add them
            // to the user unclaimed rewards for the first user collateral denom.
            let mut unclaimed_rewards = user_unclaimed_rewards.remove(&user).unwrap_or_default();

            if user_asset_index != asset_incentive.index {
                // Compute user accrued rewards
                let asset_accrued_rewards = v1_state::helpers::compute_user_accrued_rewards(
                    collateral.amount_scaled,
                    user_asset_index,
                    asset_incentive.index,
                )?;

                unclaimed_rewards += asset_accrued_rewards;
            }

            if !unclaimed_rewards.is_zero() {
                // Update user unclaimed rewards
                USER_UNCLAIMED_REWARDS.save(
                    deps.storage,
                    ((&user, ""), &denom, mars_denom),
                    &unclaimed_rewards,
                )?;
            }

            // Update user asset index
            USER_ASSET_INDICES.save(
                deps.storage,
                ((&user, ""), &denom, mars_denom),
                &asset_incentive.index,
            )?;
        }
    }

    Ok(())
}
