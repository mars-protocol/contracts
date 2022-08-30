#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Addr, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, WasmMsg,
};
use mars_outpost::address_provider::MarsContract;
use mars_outpost::{address_provider, red_bank};

use mars_outpost::error::MarsError;
use mars_outpost::helpers::option_string_to_addr;

use mars_outpost::liquidation_filter::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use mars_outpost::liquidation_filter::{Config, Liquidate};
use mars_outpost::red_bank::UserHealthStatus;

use crate::error::ContractError;
use crate::state::CONFIG;

// INIT

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        address_provider: deps.api.addr_validate(&msg.address_provider)?,
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
        ExecuteMsg::UpdateConfig {
            owner,
            address_provider,
        } => Ok(execute_update_config(deps, env, info, owner, address_provider)?),
        ExecuteMsg::LiquidateMany {
            liquidations,
        } => execute_liquidate(deps, info, liquidations),
    }
}

fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
    address_provider: Option<String>,
) -> Result<Response, MarsError> {
    let mut config = CONFIG.load(deps.storage)?;

    if info.sender != config.owner {
        return Err(MarsError::Unauthorized {});
    };

    config.owner = option_string_to_addr(deps.api, owner, config.owner)?;
    config.address_provider =
        option_string_to_addr(deps.api, address_provider, config.address_provider)?;

    CONFIG.save(deps.storage, &config)?;

    let response =
        Response::new().add_attribute("action", "outposts/mars-liquidation-filter/update_config");

    Ok(response)
}

fn execute_liquidate(
    deps: DepsMut,
    info: MessageInfo,
    liquidations: Vec<Liquidate>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let red_bank_addr = address_provider::helpers::query_address(
        deps.as_ref(),
        &config.address_provider,
        MarsContract::RedBank,
    )?;

    let mut messages = vec![];
    for liquidate in liquidations {
        let coin = info.funds.iter().find(|&c| c.denom == liquidate.debt_denom.clone()).ok_or(
            ContractError::RequiredCoin {
                denom: liquidate.debt_denom.clone(),
            },
        )?;

        let user_position_response =
            query_user_position(deps.as_ref(), &red_bank_addr, &liquidate.user_address)?;

        if let UserHealthStatus::Borrowing {
            liq_threshold_hf,
            ..
        } = user_position_response.health_status
        {
            if liq_threshold_hf < Decimal::one() {
                let liq_msg = to_red_bank_liquidate_msg(&red_bank_addr, &liquidate, coin)?;
                messages.push(liq_msg);
            }
        }
    }

    let response = Response::new()
        .add_attributes(vec![attr("action", "outposts/mars-liquidation-filter/liquidate_many")])
        .add_messages(messages);

    Ok(response)
}

fn query_user_position(
    deps: Deps<impl cosmwasm_std::CustomQuery>,
    red_bank_addr: &Addr,
    user: &str,
) -> StdResult<red_bank::UserPositionResponse> {
    let res: red_bank::UserPositionResponse = deps.querier.query_wasm_smart(
        red_bank_addr,
        &red_bank::QueryMsg::UserPosition {
            user_address: user.to_string(),
        },
    )?;

    Ok(res)
}

fn to_red_bank_liquidate_msg(
    red_bank_addr: &Addr,
    liquidate: &Liquidate,
    coin: &Coin,
) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: red_bank_addr.into(),
        msg: to_binary(&red_bank::ExecuteMsg::Liquidate {
            collateral_denom: liquidate.collateral_denom.clone(),
            debt_denom: liquidate.debt_denom.clone(),
            user_address: liquidate.user_address.clone(),
            receive_ma_token: liquidate.receive_ma_token,
        })?,
        funds: vec![coin.clone()],
    }))
}

// QUERIES

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}
