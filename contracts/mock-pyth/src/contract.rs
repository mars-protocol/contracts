use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    StdResult,
};
use pyth_sdk_cw::{Price, PriceFeed, PriceFeedResponse, PriceIdentifier, QueryMsg};

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::default())
}

#[entry_point]
pub fn execute(_deps: DepsMut, _env: Env, _info: MessageInfo, _msg: Empty) -> StdResult<Response> {
    Ok(Response::default())
}

#[entry_point]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::PriceFeed {
            id,
        } => to_json_binary(&mocked_price_feed(deps, id)?),
        _ => panic!("Unsupported query!"),
    }
}

fn mocked_price_feed(_deps: Deps, id: PriceIdentifier) -> StdResult<PriceFeedResponse> {
    let price_feed_response = PriceFeedResponse {
        price_feed: PriceFeed::new(
            id,
            Price {
                price: 680000,
                conf: 510000,
                expo: -5,
                publish_time: 1571797419,
            },
            Price {
                price: 681000,
                conf: 400000,
                expo: -5,
                publish_time: 1571797419,
            },
        ),
    };

    Ok(price_feed_response)
}
