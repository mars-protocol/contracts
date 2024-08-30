use cosmwasm_std::{
    entry_point, to_json_binary, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response,
    StdError, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw_storage_plus::Bound;
use mars_types::burn::{
    BurntAmountResponse, BurntAmountsResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
};

use crate::{
    error::{ContractError, ContractResult},
    state::BURNT_AMOUNTS,
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> ContractResult<Response> {
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::BurnFunds {
            denom,
        } => execute_burn_funds(deps, env, denom),
    }
}

pub fn execute_burn_funds(deps: DepsMut, env: Env, denom: String) -> ContractResult<Response> {
    let contract_address = env.contract.address;
    let balance = deps.querier.query_balance(contract_address, denom.clone())?;

    if balance.amount.is_zero() {
        return Err(ContractError::Std(StdError::generic_err(format!(
            "No funds to burn for denom: {}",
            denom
        ))));
    }

    let burn_msg = BankMsg::Burn {
        amount: vec![balance.clone()],
    };

    BURNT_AMOUNTS.update(deps.storage, &denom, |existing| -> StdResult<Uint128> {
        Ok(existing.unwrap_or_default() + balance.amount)
    })?;

    Ok(Response::new()
        .add_message(burn_msg)
        .add_attribute("action", "burn_funds")
        .add_attribute("denom", denom)
        .add_attribute("amount", balance.amount.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBurntAmount {
            denom,
        } => to_json_binary(&query_burnt_amount(deps, denom)?),
        QueryMsg::GetAllBurntAmounts {
            start_after,
            limit,
        } => to_json_binary(&query_all_burnt_amounts(deps, start_after, limit)?),
    }
}

fn query_burnt_amount(deps: Deps, denom: String) -> StdResult<BurntAmountResponse> {
    let amount = BURNT_AMOUNTS.may_load(deps.storage, &denom)?.unwrap_or_default();
    Ok(BurntAmountResponse {
        denom,
        amount,
    })
}

fn query_all_burnt_amounts(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u8>,
) -> StdResult<BurntAmountsResponse> {
    const MAX_LIMIT: u8 = 30;
    const DEFAULT_LIMIT: u8 = 10;

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|denom| Bound::ExclusiveRaw(denom.into_bytes()));

    let burnt_amounts: StdResult<Vec<BurntAmountResponse>> = BURNT_AMOUNTS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (denom, amount) = item?;
            Ok(BurntAmountResponse {
                denom,
                amount,
            })
        })
        .collect();

    Ok(BurntAmountsResponse {
        burnt_amounts: burnt_amounts?,
    })
}
