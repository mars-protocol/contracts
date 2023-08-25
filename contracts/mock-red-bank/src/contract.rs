#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use mars_red_bank_types::red_bank;

use crate::{
    execute::{borrow, deposit, init_asset, repay, withdraw},
    query::{query_collateral, query_collaterals, query_debt, query_market},
};

#[cfg_attr(not(feature = "library"), entry_point)]
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
    env: Env,
    info: MessageInfo,
    msg: red_bank::ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        red_bank::ExecuteMsg::InitAsset {
            denom,
            params,
        } => init_asset(deps, env, denom, params),
        red_bank::ExecuteMsg::Borrow {
            denom,
            amount,
            ..
        } => borrow(deps, info, denom, amount),
        red_bank::ExecuteMsg::Repay {
            ..
        } => repay(deps, info),
        red_bank::ExecuteMsg::Deposit {
            account_id,
        } => deposit(deps, info, account_id),
        red_bank::ExecuteMsg::Withdraw {
            denom,
            amount,
            account_id,
            liquidation_related,
            ..
        } => {
            withdraw(deps, info, &denom, &amount, account_id, liquidation_related.unwrap_or(false))
        }
        _ => unimplemented!("Msg not supported!"),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: red_bank::QueryMsg) -> StdResult<Binary> {
    match msg {
        red_bank::QueryMsg::Market {
            denom,
        } => to_binary(&query_market(deps, denom)?),
        red_bank::QueryMsg::UserDebt {
            user,
            denom,
        } => to_binary(&query_debt(deps, user, denom)?),
        red_bank::QueryMsg::UserCollateral {
            user,
            account_id,
            denom,
        } => to_binary(&query_collateral(deps, user, account_id, denom)?),
        red_bank::QueryMsg::UserCollaterals {
            user,
            account_id,
            ..
        } => to_binary(&query_collaterals(deps, user, account_id)?),
        _ => unimplemented!("Query not supported!"),
    }
}
