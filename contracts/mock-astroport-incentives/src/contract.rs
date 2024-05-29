use astroport::incentives::{ExecuteMsg, QueryMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult};
#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Deposit {
            recipient: None,
        } => todo!("Deposit"),
        ExecuteMsg::Withdraw {
            lp_token,
            amount,
        } => todo!("Withdraw"),
        ExecuteMsg::ClaimRewards {
            lp_tokens,
        } => todo!("Claim rewards"),
        _ => unimplemented!("Msg not supported!"),
    }
}

#[entry_point]
pub fn query(deps: Deps, _: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::PendingRewards {
            lp_token,
            user,
        } => todo!("Pending rewards"),
        QueryMsg::Deposit {
            lp_token,
            user,
        } => todo!("Deposits"),
        _ => panic!("Unsupported query!"),
    }
}
