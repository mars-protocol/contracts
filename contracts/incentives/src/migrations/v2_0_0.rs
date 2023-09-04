use std::collections::HashMap;

use cosmwasm_std::{DepsMut, Env, Order, Response, StdResult, Uint128};
use cw2::{assert_contract_version, set_contract_version};
use mars_owner::OwnerInit;
use mars_red_bank_types::incentives::{Config, IncentiveState, V2Updates};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION, MIN_EPOCH_DURATION},
    error::ContractError,
    state::{CONFIG, EPOCH_DURATION, INCENTIVE_STATES, OWNER, USER_ASSET_INDICES},
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

    // EPOCH_DURATION not existent in v1, initializing
    if updates.epoch_duration < MIN_EPOCH_DURATION {
        return Err(ContractError::EpochDurationTooShort {
            min_epoch_duration: MIN_EPOCH_DURATION,
        });
    }
    EPOCH_DURATION.save(deps.storage, &updates.epoch_duration)?;

    migrate_idx(&mut deps, env, &old_config_state.mars_denom)?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}

fn migrate_idx(deps: &mut DepsMut, env: Env, mars_denom: &str) -> Result<(), ContractError> {
    let current_block_time = env.block.time.seconds();

    let config = CONFIG.load(deps.storage)?;

    let red_bank_addr = mars_red_bank_types::address_provider::helpers::query_contract_addr(
        deps.as_ref(),
        &config.address_provider,
        mars_red_bank_types::address_provider::MarsAddressType::RedBank,
    )?;

    let asset_incentives: StdResult<HashMap<_, _>> =
        v1_state::ASSET_INCENTIVES.range(deps.storage, None, None, Order::Ascending).collect();
    let mut asset_incentives = asset_incentives?;

    for (denom, asset_incentive) in asset_incentives.iter_mut() {
        let market: mars_red_bank_types_old::red_bank::Market = deps.querier.query_wasm_smart(
            red_bank_addr.clone(),
            &mars_red_bank_types_old::red_bank::QueryMsg::Market {
                denom: denom.clone(),
            },
        )?;

        mars_incentives_old::helpers::update_asset_incentive_index(
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

    let user_asset_indices: StdResult<Vec<_>> =
        v1_state::USER_ASSET_INDICES.range(deps.storage, None, None, Order::Ascending).collect();
    let user_asset_indices = user_asset_indices?;

    let user_unclaimed_rewards: StdResult<HashMap<_, _>> = v1_state::USER_UNCLAIMED_REWARDS
        .range(deps.storage, None, None, Order::Ascending)
        .collect();
    let mut user_unclaimed_rewards = user_unclaimed_rewards?;

    for ((user, denom), user_asset_index) in user_asset_indices {
        let collateral: mars_red_bank_types_old::red_bank::UserCollateralResponse =
            deps.querier.query_wasm_smart(
                red_bank_addr.clone(),
                &mars_red_bank_types_old::red_bank::QueryMsg::UserCollateral {
                    user: user.to_string(),
                    denom: denom.clone(),
                },
            )?;

        // If user's balance is 0 there should be no rewards to accrue, so we don't care about
        // updating indexes. If the user's balance changes, the indexes will be updated correctly at
        // that point in time.
        if collateral.amount_scaled.is_zero() {
            continue;
        }

        let denom_idx = asset_incentives.get(&denom);
        if let Some(asset_incentive) = denom_idx {
            if user_asset_index != asset_incentive.index {
                // Compute user accrued rewards
                let asset_accrued_rewards =
                    mars_incentives_old::helpers::compute_user_accrued_rewards(
                        collateral.amount_scaled,
                        user_asset_index,
                        asset_incentive.index,
                    )?;

                // Update user unclaimed rewards
                *user_unclaimed_rewards.entry(user.clone()).or_insert_with(Uint128::zero) +=
                    asset_accrued_rewards;
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
