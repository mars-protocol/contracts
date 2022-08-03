#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult,
};
use cw_storage_plus::Bound;

use osmo_bindings::OsmosisQuery;

use mars_outpost::error::MarsError;
use mars_outpost::helpers::option_string_to_addr;
use mars_outpost::oracle::{Config, InstantiateMsg, PriceResponse, QueryMsg};

use crate::error::ContractResult;
use crate::helpers;
use crate::msg::{ExecuteMsg, PriceSource, PriceSourceResponse};
use crate::state::{CONFIG, PRICE_SOURCES};

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<impl cosmwasm_std::CustomQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    CONFIG.save(
        deps.storage,
        &Config {
            owner: deps.api.addr_validate(&msg.owner)?,
        },
    )?;

    Ok(Response::default())
}

// HANDLERS

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
        } => update_config(deps, info.sender, owner),
        ExecuteMsg::SetPriceSource {
            denom,
            price_source,
        } => set_price_source(deps, info.sender, denom, price_source),
    }
}

pub fn update_config(
    deps: DepsMut<impl cosmwasm_std::CustomQuery>,
    sender: Addr,
    owner: Option<String>,
) -> ContractResult<Response> {
    let mut cfg = CONFIG.load(deps.storage)?;
    if sender != cfg.owner {
        return Err(MarsError::Unauthorized {}.into());
    };

    cfg.owner = option_string_to_addr(deps.api, owner, cfg.owner)?;

    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new().add_attribute("action", "mars/oracle/update_config"))
}

pub fn set_price_source(
    deps: DepsMut<OsmosisQuery>,
    sender: Addr,
    denom: String,
    price_source: PriceSource,
) -> ContractResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;
    if sender != cfg.owner {
        return Err(MarsError::Unauthorized {}.into());
    }

    // for spot we must make sure the osmosis pool indicated by `pool_id` contains exactly two assets,
    // and they are OSMO and `denom`
    if let PriceSource::Spot {
        pool_id,
    } = &price_source
    {
        helpers::assert_osmosis_pool_assets(&deps.querier, *pool_id, &denom)?;
    }

    PRICE_SOURCES.save(deps.storage, denom.clone(), &price_source)?;

    Ok(Response::new()
        .add_attribute("action", "mars/oracle/set_price_source")
        .add_attribute("denom", denom)
        .add_attribute("price_source", price_source.to_string()))
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<OsmosisQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::PriceSource {
            denom,
        } => to_binary(&query_price_source(deps, denom)?),
        QueryMsg::PriceSources {
            start_after,
            limit,
        } => to_binary(&query_price_sources(deps, start_after, limit)?),
        QueryMsg::Price {
            denom,
        } => to_binary(&query_price(deps, denom)?),
        QueryMsg::Prices {
            start_after,
            limit,
        } => to_binary(&query_prices(deps, start_after, limit)?),
    }
}

pub fn query_config(deps: Deps<impl cosmwasm_std::CustomQuery>) -> StdResult<Config<String>> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(Config {
        owner: cfg.owner.to_string(),
    })
}

pub fn query_price_source(
    deps: Deps<impl cosmwasm_std::CustomQuery>,
    denom: String,
) -> StdResult<PriceSourceResponse> {
    Ok(PriceSourceResponse {
        denom: denom.clone(),
        price_source: PRICE_SOURCES.load(deps.storage, denom)?,
    })
}

pub fn query_price_sources(
    deps: Deps<impl cosmwasm_std::CustomQuery>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<PriceSourceResponse>> {
    let start = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    PRICE_SOURCES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(PriceSourceResponse {
                denom: k,
                price_source: v,
            })
        })
        .collect()
}

pub fn query_price(deps: Deps<OsmosisQuery>, denom: String) -> StdResult<PriceResponse> {
    let price_source = PRICE_SOURCES.load(deps.storage, denom.clone())?;
    Ok(PriceResponse {
        denom: denom.clone(),
        price: helpers::query_price_with_source(&deps.querier, denom, price_source)?,
    })
}

pub fn query_prices(
    deps: Deps<OsmosisQuery>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<PriceResponse>> {
    let start = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    PRICE_SOURCES
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(PriceResponse {
                denom: k.clone(),
                price: helpers::query_price_with_source(&deps.querier, k, v)?,
            })
        })
        .collect()
}
