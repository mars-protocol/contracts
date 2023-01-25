use cosmwasm_std::{
    coin, BankMsg, CosmosMsg, DepsMut, MessageInfo, Response, StdError, StdResult, Uint128,
};
use cw_utils::one_coin;

use crate::{
    helpers::{load_collateral_amount, load_debt_amount},
    state::{COLLATERAL_AMOUNT, DEBT_AMOUNT},
};

pub fn borrow(
    deps: DepsMut,
    info: MessageInfo,
    denom: String,
    amount: Uint128,
) -> StdResult<Response> {
    let debt_amount = load_debt_amount(deps.storage, &info.sender, &denom)?;

    DEBT_AMOUNT.save(
        deps.storage,
        (info.sender.clone(), denom.clone()),
        &debt_amount.checked_add(amount)?.checked_add(Uint128::new(1))?, // The extra unit is simulated accrued interest
    )?;

    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![coin(amount.u128(), denom)],
    });

    Ok(Response::new().add_message(transfer_msg))
}

pub fn repay(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    let coin_sent =
        one_coin(&info).map_err(|_| StdError::generic_err("Repay coin reqs not met"))?;
    let debt_amount = load_debt_amount(deps.storage, &info.sender, &coin_sent.denom)?;

    DEBT_AMOUNT.save(
        deps.storage,
        (info.sender, coin_sent.denom.clone()),
        &debt_amount.checked_sub(coin_sent.amount)?,
    )?;

    Ok(Response::new())
}

pub fn deposit(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    let to_deposit =
        one_coin(&info).map_err(|_| StdError::generic_err("Deposit coin reqs not met"))?;
    let collateral_amount = load_collateral_amount(deps.storage, &info.sender, &to_deposit.denom)?;

    COLLATERAL_AMOUNT.save(
        deps.storage,
        (info.sender, to_deposit.denom.clone()),
        &collateral_amount.checked_add(to_deposit.amount)?.checked_add(Uint128::new(1))?, // The extra unit is simulated accrued yield
    )?;

    Ok(Response::new())
}
