use cosmwasm_std::{
    entry_point, to_binary, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, Event, MessageInfo,
    Response, StdError, StdResult, WasmMsg,
};

use mars_core::asset::Asset;
use mars_core::helpers::cw20_get_total_supply;
use mars_core::ma_token::msg::ExecuteMsg as MaTokenExecuteMsg;

use mars_red_bank::state::MARKETS;

use crate::helpers::{build_transfer_asset_msg, cw20_get_owners_balances, get_asset_balance};
use crate::msg::ExecuteMsg;

#[entry_point]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Err(StdError::generic_err("`instantiate` is not implemented"))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Refund { asset } => refund(deps, env, asset),
    }
}

pub fn refund(deps: DepsMut, env: Env, asset: Asset) -> StdResult<Response> {
    let (asset_label, asset_ref, _) = asset.get_attributes();
    let market = MARKETS.load(deps.storage, &asset_ref)?;

    // to initiate the refund, all borrowings of the specified asset must have been repaid. that is,
    // `debt_total_scaled` parameter in its `Market` must be zero
    if !market.debt_total_scaled.is_zero() {
        return Err(StdError::generic_err(format!(
            "`debt_total_scaled` must be zero before a refund a be initiated; current value: {}",
            market.debt_total_scaled
        )));
    }

    // query:
    // - the amount of this asset held by Red Bank
    // - the maToken's total supply
    // - grab the first 10 holders of the asset's corresponding maToken and their respective balances
    let mut total_amount_to_refund =
        get_asset_balance(&deps.querier, &asset, &env.contract.address)?;
    let mut ma_token_supply =
        cw20_get_total_supply(&deps.querier, market.ma_token_address.clone())?;
    let owners_balances =
        cw20_get_owners_balances(&deps.querier, deps.api, &market.ma_token_address)?;

    // for each maToken owner, calculate how much tokens they should be refunded; create the messages
    // to burn their maToken and tranfer funds
    let mut msgs: Vec<CosmosMsg> = vec![];
    let mut events: Vec<Event> = vec![];
    for (owner_addr, balance) in owners_balances {
        let amount_to_refund = total_amount_to_refund.multiply_ratio(balance, ma_token_supply);
        total_amount_to_refund -= amount_to_refund;
        ma_token_supply -= balance;

        // burn maToken
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: market.ma_token_address.to_string(),
            msg: to_binary(&MaTokenExecuteMsg::Burn {
                user: owner_addr.to_string(),
                amount: balance,
            })?,
            funds: vec![],
        }));

        // refund asset
        msgs.push(build_transfer_asset_msg(
            &asset,
            amount_to_refund,
            &owner_addr,
        )?);

        // event log
        events.push(
            Event::new("mars_red_bank/refunded")
                .add_attribute("user", &owner_addr)
                .add_attribute("asset", &asset_label)
                .add_attribute("asset_amount", amount_to_refund)
                .add_attribute("matoken_burned", balance),
        )
    }

    Ok(Response::new().add_messages(msgs).add_events(events))
}

#[entry_point]
pub fn query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
    Err(StdError::generic_err("`query` is not implemented"))
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: Empty) -> StdResult<Response> {
    Ok(Response::new())
}
