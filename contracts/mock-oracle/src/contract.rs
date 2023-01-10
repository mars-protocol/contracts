#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use mars_outpost::oracle::PriceResponse;

use crate::{
    msg::{CoinPrice, ExecuteMsg, InstantiateMsg, QueryMsg},
    state::COIN_PRICE,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    for item in msg.prices {
        COIN_PRICE.save(deps.storage, item.denom, &item.price)?
    }
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::ChangePrice(item) => change_price(deps, item),
    }
}

fn change_price(deps: DepsMut, coin: CoinPrice) -> StdResult<Response> {
    COIN_PRICE.save(deps.storage, coin.denom, &coin.price)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Price {
            denom,
        } => to_binary(&query_price(deps, denom)?),
    }
}

fn query_price(deps: Deps, denom: String) -> StdResult<PriceResponse> {
    let price = COIN_PRICE.load(deps.storage, denom.clone())?;
    Ok(PriceResponse {
        denom,
        price,
    })
}
