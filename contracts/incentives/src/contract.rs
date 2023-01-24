#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coins, to_binary, Addr, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Response, StdResult, Uint128,
};
use mars_outpost::{
    address_provider::{self, MarsAddressType},
    error::MarsError,
    helpers::{option_string_to_addr, validate_native_denom},
    incentives::{
        AssetIncentive, AssetIncentiveResponse, Config, ConfigResponse, ExecuteMsg, InstantiateMsg,
        QueryMsg,
    },
    red_bank,
};
use mars_owner::{OwnerInit::SetInitialOwner, OwnerUpdate};

use crate::{
    error::ContractError,
    helpers::{
        compute_user_accrued_rewards, compute_user_unclaimed_rewards, update_asset_incentive_index,
    },
    state::{ASSET_INCENTIVES, CONFIG, OWNER, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS},
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
            denom,
            emission_per_second,
            start_time,
            duration,
        } => execute_set_asset_incentive(
            deps,
            env,
            info,
            denom,
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
        ExecuteMsg::ClaimRewards {} => execute_claim_rewards(deps, env, info),
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
    denom: String,
    emission_per_second: Option<Uint128>,
    start_time: Option<u64>,
    duration: Option<u64>,
) -> Result<Response, ContractError> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    validate_native_denom(&denom)?;

    let current_block_time = env.block.time.seconds();
    let new_asset_incentive = match ASSET_INCENTIVES.may_load(deps.storage, &denom)? {
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

            let red_bank_addr = address_provider::helpers::query_address(
                deps.as_ref(),
                &config.address_provider,
                MarsAddressType::RedBank,
            )?;

            let market: red_bank::Market = deps.querier.query_wasm_smart(
                red_bank_addr,
                &red_bank::QueryMsg::Market {
                    denom: denom.clone(),
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

    ASSET_INCENTIVES.save(deps.storage, &denom, &new_asset_incentive)?;

    let response = Response::new().add_attributes(vec![
        attr("action", "outposts/incentives/set_asset_incentive"),
        attr("denom", denom),
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
    denom: String,
    user_amount_scaled_before: Uint128,
    total_amount_scaled_before: Uint128,
) -> Result<Response, ContractError> {
    // this method can only be invoked by the Red Bank contract
    let red_bank_addr = query_red_bank_address(deps.as_ref())?;
    if info.sender != red_bank_addr {
        return Err(MarsError::Unauthorized {}.into());
    }

    let mut asset_incentive = match ASSET_INCENTIVES.may_load(deps.storage, &denom)? {
        // If there are no incentives,
        // an empty successful response is returned as the
        // success of the call is needed for the call that triggered the change to
        // succeed and be persisted to state.
        None => return Ok(Response::default()),

        Some(ai) => ai,
    };

    update_asset_incentive_index(
        &mut asset_incentive,
        total_amount_scaled_before,
        env.block.time.seconds(),
    )?;
    ASSET_INCENTIVES.save(deps.storage, &denom, &asset_incentive)?;

    // Check if user has accumulated uncomputed rewards (which means index is not up to date)
    let user_asset_index_key = USER_ASSET_INDICES.key((&user_addr, &denom));

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
                &user_addr,
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

    let response = Response::new().add_attributes(vec![
        attr("action", "outposts/incentives/balance_change"),
        attr("denom", denom),
        attr("user", user_addr),
        attr("rewards_accrued", accrued_rewards),
        attr("asset_index", asset_incentive.index.to_string()),
    ]);

    Ok(response)
}

pub fn execute_claim_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let red_bank_addr = query_red_bank_address(deps.as_ref())?;
    let user_addr = info.sender;
    let (total_unclaimed_rewards, user_asset_incentive_statuses_to_update) =
        compute_user_unclaimed_rewards(deps.as_ref(), &env.block, &red_bank_addr, &user_addr)?;

    // Commit updated asset_incentives and user indexes
    for user_asset_incentive_status in user_asset_incentive_statuses_to_update {
        let asset_incentive_updated = user_asset_incentive_status.asset_incentive_updated;

        ASSET_INCENTIVES.save(
            deps.storage,
            &user_asset_incentive_status.denom,
            &asset_incentive_updated,
        )?;

        if asset_incentive_updated.index != user_asset_incentive_status.user_index_current {
            USER_ASSET_INDICES.save(
                deps.storage,
                (&user_addr, &user_asset_incentive_status.denom),
                &asset_incentive_updated.index,
            )?
        }
    }

    // clear unclaimed rewards
    USER_UNCLAIMED_REWARDS.save(deps.storage, &user_addr, &Uint128::zero())?;

    let mut response = Response::new();
    if !total_unclaimed_rewards.is_zero() {
        let config = CONFIG.load(deps.storage)?;
        // Build message to send mars to the user
        response = response.add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: user_addr.to_string(),
            amount: coins(total_unclaimed_rewards.u128(), config.mars_denom),
        }));
    };

    response = response.add_attributes(vec![
        attr("action", "outposts/incentives/claim_rewards"),
        attr("user", user_addr),
        attr("mars_rewards", total_unclaimed_rewards),
    ]);

    Ok(response)
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

    let response = Response::new().add_attribute("action", "outposts/incentives/update_config");

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
            denom,
        } => to_binary(&query_asset_incentive(deps, denom)?),
        QueryMsg::UserUnclaimedRewards {
            user,
        } => to_binary(&query_user_unclaimed_rewards(deps, env, user)?),
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

pub fn query_asset_incentive(deps: Deps, denom: String) -> StdResult<AssetIncentiveResponse> {
    let option_asset_incentive = ASSET_INCENTIVES.may_load(deps.storage, &denom)?;
    Ok(AssetIncentiveResponse {
        asset_incentive: option_asset_incentive,
    })
}

pub fn query_user_unclaimed_rewards(deps: Deps, env: Env, user: String) -> StdResult<Uint128> {
    let red_bank_addr = query_red_bank_address(deps)?;
    let user_addr = deps.api.addr_validate(&user)?;
    let (unclaimed_rewards, _) =
        compute_user_unclaimed_rewards(deps, &env.block, &red_bank_addr, &user_addr)?;

    Ok(unclaimed_rewards)
}

fn query_red_bank_address(deps: Deps) -> StdResult<Addr> {
    let config = CONFIG.load(deps.storage)?;
    address_provider::helpers::query_address(
        deps,
        &config.address_provider,
        MarsAddressType::RedBank,
    )
}
