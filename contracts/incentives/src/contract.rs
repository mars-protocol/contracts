#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, coins, to_binary, Addr, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Response, StdResult, Uint128,
};

use mars_outpost::address_provider::MarsAddressType;
use mars_outpost::error::MarsError;
use mars_outpost::helpers::option_string_to_addr;

use mars_outpost::incentives::{AssetIncentive, AssetIncentiveResponse, Config};
use mars_outpost::incentives::{ExecuteMsg, InstantiateMsg, QueryMsg};
use mars_outpost::{address_provider, red_bank};

use crate::error::ContractError;
use crate::helpers::{
    asset_incentive_update_index, compute_user_unclaimed_rewards, user_compute_accrued_rewards,
};
use crate::state::{ASSET_INCENTIVES, CONFIG, USER_ASSET_INDICES, USER_UNCLAIMED_REWARDS};

pub const CONTRACT_NAME: &str = "crates.io:mars-incentives";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
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
        } => execute_set_asset_incentive(deps, env, info, denom, emission_per_second),
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
            owner,
            address_provider,
            mars_denom,
        } => Ok(execute_update_config(deps, env, info, owner, address_provider, mars_denom)?),
    }
}

pub fn execute_set_asset_incentive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    emission_per_second: Uint128,
) -> Result<Response, ContractError> {
    // only owner can call this
    let config = CONFIG.load(deps.storage)?;
    let owner = config.owner;
    if info.sender != owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let red_bank_addr = address_provider::helpers::query_address(
        deps.as_ref(),
        &config.address_provider,
        MarsAddressType::RedBank,
    )?;

    let new_asset_incentive = match ASSET_INCENTIVES.may_load(deps.storage, &denom)? {
        Some(mut asset_incentive) => {
            let market: red_bank::Market = deps.querier.query_wasm_smart(
                red_bank_addr,
                &red_bank::QueryMsg::Market {
                    denom: denom.clone(),
                },
            )?;

            // Update index up to now
            asset_incentive_update_index(
                &mut asset_incentive,
                market.collateral_total_scaled,
                env.block.time.seconds(),
            )?;

            // Set new emission
            asset_incentive.emission_per_second = emission_per_second;

            asset_incentive
        }
        None => AssetIncentive {
            emission_per_second,
            index: Decimal::zero(),
            last_updated: env.block.time.seconds(),
        },
    };

    ASSET_INCENTIVES.save(deps.storage, &denom, &new_asset_incentive)?;

    let response = Response::new().add_attributes(vec![
        attr("action", "outposts/incentives/set_asset_incentive"),
        attr("denom", denom),
        attr("emission_per_second", emission_per_second),
    ]);
    Ok(response)
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

    asset_incentive_update_index(
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
        accrued_rewards = user_compute_accrued_rewards(
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
    owner: Option<String>,
    address_provider: Option<String>,
    mars_denom: Option<String>,
) -> Result<Response, MarsError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {});
    };

    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;
    config.address_provider =
        option_string_to_addr(deps.api, address_provider, config.address_provider)?;
    config.mars_denom = mars_denom.unwrap_or(config.mars_denom);

    CONFIG.save(deps.storage, &config)?;

    let response = Response::new().add_attribute("action", "outposts/incentives/update_config");

    Ok(response)
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

pub fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
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
