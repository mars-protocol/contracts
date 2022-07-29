#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, IbcMsg, IbcTimeout,
    IbcTimeoutBlock, MessageInfo, Order, Response, StdResult, Uint128, WasmMsg,
};
use cw_storage_plus::Bound;

use mars_outpost::address_provider::{self, MarsContract};
use mars_outpost::error::MarsError;
use mars_outpost::helpers::option_string_to_addr;
use mars_outpost::protocol_rewards_collector::{
    Config, CreateOrUpdateConfig, InstantiateMsg, QueryMsg, RouteResponse, RoutesResponse,
};
use mars_outpost::red_bank;

use osmo_bindings::{OsmosisMsg, OsmosisQuery};

use crate::error::{ContractError, ContractResult};
use crate::helpers::{stringify_option_amount, unwrap_option_amount};
use crate::msg::ExecuteMsg;
use crate::state::{CONFIG, ROUTES};
use crate::swap::Route;

const DEFAULT_LIMIT: u32 = 5;
const MAX_LIMIT: u32 = 10;

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    let cfg = msg.check(deps.api)?;
    cfg.validate()?;

    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::default())
}

// HANDLERS

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<OsmosisQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response<OsmosisMsg>> {
    match msg {
        ExecuteMsg::UpdateConfig(new_cfg) => update_config(deps, info.sender, new_cfg),
        ExecuteMsg::SetRoute {
            denom_in,
            denom_out,
            route,
        } => set_route(deps, info.sender, denom_in, denom_out, route),
        ExecuteMsg::WithdrawFromRedBank {
            denom,
            amount,
        } => withdraw_from_red_bank(deps, denom, amount),
        ExecuteMsg::DistributeRewards {
            denom,
            amount,
        } => distribute_rewards(deps, env, denom, amount),
        ExecuteMsg::SwapAsset {
            denom,
            amount,
        } => swap_asset(deps, env, denom, amount),
        ExecuteMsg::ExecuteCosmosMsg(cosmos_msg) => {
            execute_cosmos_msg(deps, info.sender, cosmos_msg)
        }
    }
}

pub fn update_config(
    deps: DepsMut<impl cosmwasm_std::CustomQuery>,
    sender: Addr,
    new_cfg: CreateOrUpdateConfig,
) -> ContractResult<Response<OsmosisMsg>> {
    let mut cfg = CONFIG.load(deps.storage)?;

    if sender != cfg.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    let CreateOrUpdateConfig {
        owner,
        address_provider,
        safety_tax_rate,
        safety_fund_denom,
        fee_collector_denom,
        channel_id,
        timeout_revision,
        timeout_blocks,
        timeout_seconds,
    } = new_cfg;

    cfg.owner = option_string_to_addr(deps.api, owner, cfg.owner)?;
    cfg.address_provider = option_string_to_addr(deps.api, address_provider, cfg.address_provider)?;
    cfg.safety_tax_rate = safety_tax_rate.unwrap_or(cfg.safety_tax_rate);
    cfg.safety_fund_denom = safety_fund_denom.unwrap_or(cfg.safety_fund_denom);
    cfg.fee_collector_denom = fee_collector_denom.unwrap_or(cfg.fee_collector_denom);
    cfg.channel_id = channel_id.unwrap_or(cfg.channel_id);
    cfg.timeout_revision = timeout_revision.unwrap_or(cfg.timeout_revision);
    cfg.timeout_blocks = timeout_blocks.unwrap_or(cfg.timeout_blocks);
    cfg.timeout_seconds = timeout_seconds.unwrap_or(cfg.timeout_seconds);

    cfg.validate()?;

    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new().add_attribute("action", "mars/rewards-collector/update_config"))
}

pub fn set_route(
    deps: DepsMut<OsmosisQuery>,
    sender: Addr,
    denom_in: String,
    denom_out: String,
    route: Route,
) -> ContractResult<Response<OsmosisMsg>> {
    let cfg = CONFIG.load(deps.storage)?;

    if sender != cfg.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    route.validate(&deps.querier, &denom_in, &denom_out)?;

    ROUTES.save(deps.storage, (denom_in.clone(), denom_out.clone()), &route)?;

    Ok(Response::new()
        .add_attribute("action", "mars/rewards-collector/set_instructions")
        .add_attribute("denom_in", denom_in)
        .add_attribute("denom_out", denom_out))
}

pub fn withdraw_from_red_bank(
    deps: DepsMut<impl cosmwasm_std::CustomQuery>,
    denom: String,
    amount: Option<Uint128>,
) -> ContractResult<Response<OsmosisMsg>> {
    let cfg = CONFIG.load(deps.storage)?;

    let red_bank_addr = address_provider::helpers::query_address(
        deps.as_ref(),
        &cfg.address_provider,
        MarsContract::RedBank,
    )?;

    // TODO: update red bank execute msg to take denom instead of asset
    let withdraw_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: red_bank_addr.to_string(),
        msg: to_binary(&red_bank::msg::ExecuteMsg::Withdraw {
            asset: mars_outpost::asset::Asset::Native {
                denom: denom.clone(),
            },
            amount,
            recipient: None,
        })?,
        funds: vec![],
    });

    Ok(Response::new()
        .add_message(withdraw_msg)
        .add_attribute("action", "mars/rewards-collector/withdraw_from_red_bank")
        .add_attribute("denom", denom)
        .add_attribute("amount", stringify_option_amount(amount)))
}

pub fn swap_asset(
    deps: DepsMut<impl cosmwasm_std::CustomQuery>,
    env: Env,
    denom: String,
    amount: Option<Uint128>,
) -> ContractResult<Response<OsmosisMsg>> {
    let cfg = CONFIG.load(deps.storage)?;

    // if amount is None, swap the total balance
    let amount_to_swap =
        unwrap_option_amount(&deps.querier, &env.contract.address, &denom, amount)?;

    // split the amount to swap between the safety fund and the fee collector
    let amount_safety_fund = amount_to_swap * cfg.safety_tax_rate;
    let amount_fee_collector = amount_to_swap.checked_sub(amount_safety_fund)?;
    let mut messages = vec![];

    if !amount_safety_fund.is_zero() {
        messages.push(
            ROUTES
                .load(deps.storage, (denom.clone(), cfg.safety_fund_denom))?
                .build_swap_msg(&denom, amount_safety_fund)?,
        );
    }

    if !amount_fee_collector.is_zero() {
        messages.push(
            ROUTES
                .load(deps.storage, (denom.clone(), cfg.fee_collector_denom))?
                .build_swap_msg(&denom, amount_fee_collector)?,
        );
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", "mars/rewards-collector/swap_asset")
        .add_attribute("denom", denom)
        .add_attribute("amount_safety_fund", amount_safety_fund)
        .add_attribute("amount_fee_collector", amount_fee_collector))
}

pub fn distribute_rewards(
    deps: DepsMut<impl cosmwasm_std::CustomQuery>,
    env: Env,
    denom: String,
    amount: Option<Uint128>,
) -> ContractResult<Response<OsmosisMsg>> {
    let cfg = CONFIG.load(deps.storage)?;

    let to_address = if denom == cfg.safety_fund_denom {
        address_provider::helpers::query_address(
            deps.as_ref(),
            &cfg.address_provider,
            MarsContract::SafetyFund,
        )?
    } else if denom == cfg.fee_collector_denom {
        address_provider::helpers::query_address(
            deps.as_ref(),
            &cfg.address_provider,
            MarsContract::FeeCollector,
        )?
    } else {
        return Err(ContractError::AssetNotEnabledForDistribution {
            denom,
        });
    };

    let amount_to_distribute =
        unwrap_option_amount(&deps.querier, &env.contract.address, &denom, amount)?;

    let transfer_msg = CosmosMsg::Ibc(IbcMsg::Transfer {
        channel_id: cfg.channel_id,
        to_address: to_address.to_string(),
        amount: Coin {
            denom: denom.clone(),
            amount: amount_to_distribute,
        },
        timeout: IbcTimeout::with_both(
            IbcTimeoutBlock {
                revision: cfg.timeout_revision,
                height: env.block.height + cfg.timeout_blocks,
            },
            env.block.time.plus_seconds(cfg.timeout_seconds),
        ),
    });

    Ok(Response::new()
        .add_message(transfer_msg)
        .add_attribute("action", "mars/rewards-collector/distribute_rewards")
        .add_attribute("denom", denom)
        .add_attribute("amount", amount_to_distribute))
}

pub fn execute_cosmos_msg(
    deps: DepsMut<impl cosmwasm_std::CustomQuery>,
    sender: Addr,
    msg: CosmosMsg<OsmosisMsg>,
) -> ContractResult<Response<OsmosisMsg>> {
    let cfg = CONFIG.load(deps.storage)?;

    if sender != cfg.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    Ok(Response::new()
        .add_message(msg)
        .add_attribute("action", "mars/rewards-collector/execute_cosmos_msg"))
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(
    deps: Deps<impl cosmwasm_std::CustomQuery>,
    _env: Env,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Route {
            denom_in,
            denom_out,
        } => to_binary(&query_route(deps, denom_in, denom_out)?),
        QueryMsg::Routes {
            start_after,
            limit,
        } => to_binary(&query_routes(deps, start_after, limit)?),
    }
}

pub fn query_config(deps: Deps<impl cosmwasm_std::CustomQuery>) -> StdResult<Config<String>> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(cfg.into())
}

pub fn query_route(
    deps: Deps<impl cosmwasm_std::CustomQuery>,
    denom_in: String,
    denom_out: String,
) -> StdResult<RouteResponse<Route>> {
    Ok(RouteResponse {
        denom_in: denom_in.clone(),
        denom_out: denom_out.clone(),
        route: ROUTES.load(deps.storage, (denom_in, denom_out))?,
    })
}

pub fn query_routes(
    deps: Deps<impl cosmwasm_std::CustomQuery>,
    start_after: Option<(String, String)>,
    limit: Option<u32>,
) -> StdResult<RoutesResponse<Route>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    ROUTES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(RouteResponse {
                denom_in: k.0,
                denom_out: k.1,
                route: v,
            })
        })
        .collect()
}
