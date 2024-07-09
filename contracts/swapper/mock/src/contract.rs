use cosmwasm_std::{
    coins, to_json_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, Response, StdError, StdResult, Uint128,
};
use mars_types::swapper::{
    EstimateExactInSwapResponse, ExecuteMsg, InstantiateMsg, QueryMsg, SwapperRoute,
};

pub const MOCK_SWAP_RESULT: Uint128 = Uint128::new(1337);

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<Empty, Empty>,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateOwner(_) => unimplemented!("not implemented"),
        ExecuteMsg::SetRoute {
            ..
        } => unimplemented!("not implemented"),
        ExecuteMsg::TransferResult {
            ..
        } => unimplemented!("not implemented"),
        ExecuteMsg::SwapExactIn {
            coin_in,
            denom_out,
            min_receive,
            route,
        } => swap_exact_in(deps, env, info, coin_in, denom_out, min_receive, route),
        ExecuteMsg::UpdateConfig {
            ..
        } => unimplemented!("not implemented"),
    }
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Owner {
            ..
        } => unimplemented!("not implemented"),
        QueryMsg::Route {
            ..
        } => unimplemented!("not implemented"),
        QueryMsg::Routes {
            ..
        } => unimplemented!("not implemented"),
        QueryMsg::EstimateExactInSwap {
            ..
        } => to_json_binary(&estimate_exact_in_swap()),
        QueryMsg::Config {
            ..
        } => unimplemented!("not implemented"),
    }
}

pub fn estimate_exact_in_swap() -> EstimateExactInSwapResponse {
    EstimateExactInSwapResponse {
        amount: MOCK_SWAP_RESULT,
    }
}

pub fn swap_exact_in(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    coin_in: Coin,
    denom_out: String,
    min_receive: Uint128,
    _route: Option<SwapperRoute>,
) -> StdResult<Response> {
    let denom_in_balance = deps.querier.query_balance(env.contract.address, coin_in.denom)?;
    if denom_in_balance.amount < coin_in.amount {
        return Err(StdError::generic_err("Did not send funds"));
    }

    let transfer_amt = if denom_out == "uosmo" {
        MOCK_SWAP_RESULT
    } else {
        coin_in.amount
    };

    if transfer_amt < min_receive {
        return Err(StdError::generic_err("Min amount not reached"));
    }

    // This is dependent on the mock env to pre-fund this contract with uosmo coins
    // simulating a swap has taken place
    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: coins(transfer_amt.u128(), denom_out),
    });

    Ok(Response::new().add_attribute("action", "transfer_result").add_message(transfer_msg))
}
