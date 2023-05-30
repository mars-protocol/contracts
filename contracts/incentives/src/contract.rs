use std::collections::HashMap;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coins, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    Event, MessageInfo, Order, Response, StdResult, Uint128,
};
use mars_owner::{OwnerInit::SetInitialOwner, OwnerUpdate};
use mars_red_bank_types::{
    address_provider::{self, MarsAddressType},
    error::MarsError,
    incentives::{
        AssetIncentive, AssetIncentiveResponse, Config, ConfigResponse, ExecuteMsg, InstantiateMsg,
        QueryMsg,
    },
    red_bank,
};
use mars_utils::helpers::{option_string_to_addr, validate_native_denom};

use crate::{
    error::ContractError,
    helpers::{
        compute_user_accrued_rewards, compute_user_unclaimed_rewards, update_asset_incentive_index,
    },
    state::{self, ASSET_INCENTIVES, CONFIG, OWNER, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS},
};

pub const CONTRACT_NAME: &str = "crates.io:mars-incentives";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    OWNER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.owner,
        },
    )?;

    let config = Config {
        address_provider: deps.api.addr_validate(&msg.address_provider)?,
        mars_denom: msg.mars_denom,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

// HANDLERS

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::SetAssetIncentive {
            collateral_denom,
            incentive_denom,
            emission_per_second,
            start_time,
            duration,
        } => execute_set_asset_incentive(
            deps,
            env,
            info,
            collateral_denom,
            incentive_denom,
            emission_per_second,
            start_time,
            duration,
        ),
        ExecuteMsg::BalanceChange {
            user_addr,
            denom,
            user_amount_scaled_before,
            total_amount_scaled_before,
        } => execute_balance_change(
            deps,
            env,
            info,
            user_addr,
            denom,
            user_amount_scaled_before,
            total_amount_scaled_before,
        ),
        ExecuteMsg::ClaimRewards {
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        } => execute_claim_rewards(
            deps,
            env,
            info,
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        ),
        ExecuteMsg::UpdateConfig {
            address_provider,
            mars_denom,
        } => Ok(execute_update_config(deps, env, info, address_provider, mars_denom)?),
        ExecuteMsg::UpdateOwner(update) => update_owner(deps, info, update),
    }
}

pub fn execute_set_asset_incentive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    collateral_denom: String,
    incentive_denom: String,
    emission_per_second: Option<Uint128>,
    start_time: Option<u64>,
    duration: Option<u64>,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    validate_native_denom(&collateral_denom)?;

    let current_block_time = env.block.time.seconds();
    let new_asset_incentive = match ASSET_INCENTIVES
        .may_load(deps.storage, (collateral_denom.clone(), incentive_denom.clone()))?
    {
        Some(mut asset_incentive) => {
            let (start_time, duration, emission_per_second) =
                validate_params_for_existing_incentive(
                    &asset_incentive,
                    emission_per_second,
                    start_time,
                    duration,
                    current_block_time,
                )?;

            let config = CONFIG.load(deps.storage)?;

            let red_bank_addr = address_provider::helpers::query_contract_addr(
                deps.as_ref(),
                &config.address_provider,
                MarsAddressType::RedBank,
            )?;

            let market: red_bank::Market = deps.querier.query_wasm_smart(
                red_bank_addr,
                &red_bank::QueryMsg::Market {
                    denom: collateral_denom.clone(),
                },
            )?;

            // Update index up to now
            update_asset_incentive_index(
                &mut asset_incentive,
                market.collateral_total_scaled,
                current_block_time,
            )?;

            // Set new emission
            asset_incentive.emission_per_second = emission_per_second;
            asset_incentive.start_time = start_time;
            asset_incentive.duration = duration;

            asset_incentive
        }
        None => {
            let (start_time, duration, emission_per_second) = validate_params_for_new_incentive(
                start_time,
                duration,
                emission_per_second,
                current_block_time,
            )?;

            AssetIncentive {
                emission_per_second,
                start_time,
                duration,
                index: Decimal::zero(),
                last_updated: current_block_time,
            }
        }
    };

    ASSET_INCENTIVES.save(
        deps.storage,
        (collateral_denom.clone(), incentive_denom.clone()),
        &new_asset_incentive,
    )?;

    let response = Response::new().add_attributes(vec![
        attr("action", "set_asset_incentive"),
        attr("collateral_denom", collateral_denom),
        attr("incentive_denom", incentive_denom),
        attr("emission_per_second", new_asset_incentive.emission_per_second),
        attr("start_time", new_asset_incentive.start_time.to_string()),
        attr("duration", new_asset_incentive.duration.to_string()),
    ]);
    Ok(response)
}

fn validate_params_for_existing_incentive(
    asset_incentive: &AssetIncentive,
    emission_per_second: Option<Uint128>,
    start_time: Option<u64>,
    duration: Option<u64>,
    current_block_time: u64,
) -> Result<(u64, u64, Uint128), ContractError> {
    let end_time = asset_incentive.start_time + asset_incentive.duration;
    let start_time = match start_time {
        // current asset incentive hasn't finished yet
        Some(_)
            if asset_incentive.start_time <= current_block_time
                && end_time >= current_block_time =>
        {
            return Err(ContractError::InvalidIncentive {
                reason: "can't modify start_time if incentive in progress".to_string(),
            })
        }
        // start_time can't be from the past
        Some(st) if st < current_block_time => {
            return Err(ContractError::InvalidIncentive {
                reason: "start_time can't be less than current block time".to_string(),
            });
        }
        // correct start_time so it can be used
        Some(st) => st,
        // previous asset incentive finished so new start_time is required
        None if end_time < current_block_time => {
            return Err(ContractError::InvalidIncentive {
                reason: "start_time is required for new incentive".to_string(),
            })
        }
        // use start_time from current asset incentive
        None => asset_incentive.start_time,
    };

    let duration = match duration {
        // can't be 0
        Some(dur) if dur == 0 => {
            return Err(ContractError::InvalidIncentive {
                reason: "duration can't be 0".to_string(),
            })
        }
        // end_time can't be decreased to the past
        Some(dur) if start_time + dur < current_block_time => {
            return Err(ContractError::InvalidIncentive {
                reason: "end_time can't be less than current block time".to_string(),
            })
        }
        // correct duration so it can be used
        Some(dur) => dur,
        // use duration from current asset incentive
        None => asset_incentive.duration,
    };

    let emission_per_second = emission_per_second.unwrap_or(asset_incentive.emission_per_second);

    Ok((start_time, duration, emission_per_second))
}

fn validate_params_for_new_incentive(
    start_time: Option<u64>,
    duration: Option<u64>,
    emission_per_second: Option<Uint128>,
    current_block_time: u64,
) -> Result<(u64, u64, Uint128), ContractError> {
    // all params are required during incentive initialization (if start_time = None then set to current block time)
    let (Some(start_time), Some(duration), Some(emission_per_second)) = (start_time, duration, emission_per_second) else {
        return Err(ContractError::InvalidIncentive {
            reason: "all params are required during incentive initialization".to_string(),
        });
    };

    // duration should be greater than 0
    if duration == 0 {
        return Err(ContractError::InvalidIncentive {
            reason: "duration can't be 0".to_string(),
        });
    }

    // start_time can't be less that current block time
    if start_time < current_block_time {
        return Err(ContractError::InvalidIncentive {
            reason: "start_time can't be less than current block time".to_string(),
        });
    }

    Ok((start_time, duration, emission_per_second))
}

pub fn execute_balance_change(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user_addr: Addr,
    collateral_denom: String,
    user_amount_scaled_before: Uint128,
    total_amount_scaled_before: Uint128,
) -> Result<Response, ContractError> {
    // this method can only be invoked by the Red Bank contract
    let red_bank_addr = query_red_bank_address(deps.as_ref())?;
    if info.sender != red_bank_addr {
        return Err(MarsError::Unauthorized {}.into());
    }

    let mut event = Event::new("mars/incentives/balance_change")
        .add_attribute("action", "balance_change")
        .add_attribute("denom", collateral_denom.clone())
        .add_attribute("user", user_addr.to_string());

    let asset_incentives = ASSET_INCENTIVES
        .prefix(collateral_denom.clone())
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;

    for (incentive_denom, mut asset_incentive) in asset_incentives {
        update_asset_incentive_index(
            &mut asset_incentive,
            total_amount_scaled_before,
            env.block.time.seconds(),
        )?;
        ASSET_INCENTIVES.save(
            deps.storage,
            (collateral_denom.clone(), incentive_denom.clone()),
            &asset_incentive,
        )?;

        // Check if user has accumulated uncomputed rewards (which means index is not up to date)
        let user_asset_index_key =
            USER_ASSET_INDICES.key((&user_addr, &collateral_denom, &incentive_denom));

        let user_asset_index =
            user_asset_index_key.may_load(deps.storage)?.unwrap_or_else(Decimal::zero);

        let mut accrued_rewards = Uint128::zero();

        if user_asset_index != asset_incentive.index {
            // Compute user accrued rewards and update state
            accrued_rewards = compute_user_accrued_rewards(
                user_amount_scaled_before,
                user_asset_index,
                asset_incentive.index,
            )?;

            // Store user accrued rewards as unclaimed
            if !accrued_rewards.is_zero() {
                USER_UNCLAIMED_REWARDS.update(
                    deps.storage,
                    (&user_addr, &incentive_denom),
                    |ur: Option<Uint128>| -> StdResult<Uint128> {
                        match ur {
                            Some(unclaimed_rewards) => Ok(unclaimed_rewards + accrued_rewards),
                            None => Ok(accrued_rewards),
                        }
                    },
                )?;
            }

            user_asset_index_key.save(deps.storage, &asset_incentive.index)?;
        }

        event = event
            .add_attribute(format!("rewards_accrued_{}", incentive_denom), accrued_rewards)
            .add_attribute(
                format!("asset_index_{}", incentive_denom),
                asset_incentive.index.to_string(),
            );
    }

    Ok(Response::new().add_event(event))
}

pub fn execute_claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    start_after_collateral_denom: Option<String>,
    start_after_incentive_denom: Option<String>,
    limit: Option<u32>,
) -> Result<Response, ContractError> {
    let red_bank_addr = query_red_bank_address(deps.as_ref())?;
    let user_addr = info.sender;

    let mut response = Response::new();
    let mut event = Event::new("mars/incentives/claim_rewards")
        .add_attribute("action", "claim_rewards")
        .add_attribute("user", user_addr.to_string());

    let asset_incentives = state::paginate_asset_incentives(
        deps.storage,
        start_after_collateral_denom,
        start_after_incentive_denom,
        limit,
    )?;
    for ((collateral_denom, incentive_denom), _) in asset_incentives {
        let (unclaimed_rewards, user_asset_incentive_statuses_to_update) =
            compute_user_unclaimed_rewards(
                deps.as_ref(),
                &env.block,
                &red_bank_addr,
                &user_addr,
                &collateral_denom,
                &incentive_denom,
            )?;

        // Commit updated asset_incentives and user indexes
        if let Some(user_asset_incentive_status) = user_asset_incentive_statuses_to_update {
            let asset_incentive_updated = user_asset_incentive_status.asset_incentive_updated;

            ASSET_INCENTIVES.save(
                deps.storage,
                (collateral_denom.clone(), incentive_denom.clone()),
                &asset_incentive_updated,
            )?;

            if asset_incentive_updated.index != user_asset_incentive_status.user_index_current {
                USER_ASSET_INDICES.save(
                    deps.storage,
                    (&user_addr, &collateral_denom, &incentive_denom),
                    &asset_incentive_updated.index,
                )?
            }
        }

        // clear unclaimed rewards
        USER_UNCLAIMED_REWARDS.save(
            deps.storage,
            (&user_addr, &incentive_denom),
            &Uint128::zero(),
        )?;

        if !unclaimed_rewards.is_zero() {
            // Build message to send the incentive to the user
            // TODO: Group all sends of the same denom in a single message?
            response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: user_addr.to_string(),
                amount: coins(unclaimed_rewards.u128(), &incentive_denom),
            }));
            event = event.add_attribute(format!("{}_rewards", incentive_denom), unclaimed_rewards);
        };
    }

    Ok(response.add_event(event))
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address_provider: Option<String>,
    mars_denom: Option<String>,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    if let Some(md) = &mars_denom {
        validate_native_denom(md)?;
    };

    let mut config = CONFIG.load(deps.storage)?;

    config.address_provider =
        option_string_to_addr(deps.api, address_provider, config.address_provider)?;
    config.mars_denom = mars_denom.unwrap_or(config.mars_denom);

    CONFIG.save(deps.storage, &config)?;

    let response = Response::new().add_attribute("action", "update_config");

    Ok(response)
}

fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    update: OwnerUpdate,
) -> Result<Response, ContractError> {
    Ok(OWNER.update(deps, info, update)?)
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::AssetIncentive {
            collateral_denom,
            incentive_denom,
        } => to_binary(&query_asset_incentive(deps, collateral_denom, incentive_denom)?),
        QueryMsg::AssetIncentives {
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        } => to_binary(&query_asset_incentives(
            deps,
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        )?),
        QueryMsg::UserUnclaimedRewards {
            user,
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        } => to_binary(&query_user_unclaimed_rewards(
            deps,
            env,
            user,
            start_after_collateral_denom,
            start_after_incentive_denom,
            limit,
        )?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let owner_state = OWNER.query(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: owner_state.owner,
        proposed_new_owner: owner_state.proposed,
        address_provider: config.address_provider,
        mars_denom: config.mars_denom,
    })
}

pub fn query_asset_incentive(
    deps: Deps,
    collateral_denom: String,
    incentive_denom: String,
) -> StdResult<AssetIncentiveResponse> {
    let asset_incentive =
        ASSET_INCENTIVES.load(deps.storage, (collateral_denom.clone(), incentive_denom.clone()))?;
    Ok(AssetIncentiveResponse::from(collateral_denom, incentive_denom, asset_incentive))
}

pub fn query_asset_incentives(
    deps: Deps,
    start_after_collateral_denom: Option<String>,
    start_after_incentive_denom: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<AssetIncentiveResponse>> {
    let asset_incentives = state::paginate_asset_incentives(
        deps.storage,
        start_after_collateral_denom,
        start_after_incentive_denom,
        limit,
    )?;

    asset_incentives
        .into_iter()
        .map(|((collateral_denom, incentive_denom), ai)| {
            Ok(AssetIncentiveResponse::from(collateral_denom, incentive_denom, ai))
        })
        .collect()
}

pub fn query_user_unclaimed_rewards(
    deps: Deps,
    env: Env,
    user: String,
    start_after_collateral_denom: Option<String>,
    start_after_incentive_denom: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<Coin>> {
    let red_bank_addr = query_red_bank_address(deps)?;
    let user_addr = deps.api.addr_validate(&user)?;

    let asset_incentives = state::paginate_asset_incentives(
        deps.storage,
        start_after_collateral_denom,
        start_after_incentive_denom,
        limit,
    )?;

    let mut total_unclaimed_rewards: HashMap<String, Uint128> = HashMap::new();

    for ((collateral_denom, incentive_denom), _) in asset_incentives {
        let (unclaimed_rewards, _) = compute_user_unclaimed_rewards(
            deps,
            &env.block,
            &red_bank_addr,
            &user_addr,
            &collateral_denom,
            &incentive_denom,
        )?;
        if let Some(x) = total_unclaimed_rewards.get_mut(&incentive_denom) {
            *x += unclaimed_rewards;
        } else {
            total_unclaimed_rewards.insert(incentive_denom, unclaimed_rewards);
        }
    }

    Ok(total_unclaimed_rewards
        .into_iter()
        .map(|(denom, amount)| Coin {
            denom,
            amount: amount.into(),
        })
        .collect())
}

fn query_red_bank_address(deps: Deps) -> StdResult<Addr> {
    let config = CONFIG.load(deps.storage)?;
    address_provider::helpers::query_contract_addr(
        deps,
        &config.address_provider,
        MarsAddressType::RedBank,
    )
}
