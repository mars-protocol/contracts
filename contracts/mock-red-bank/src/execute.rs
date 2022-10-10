use cosmwasm_std::{coin, BankMsg, CosmosMsg, DepsMut, MessageInfo, Response, StdResult, Uint128};

use crate::helpers::load_debt_amount;
use crate::state::DEBT_AMOUNT;

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
        &debt_amount
            .checked_add(amount)?
            .checked_add(Uint128::new(1))?, // The extra unit is simulated accrued interest
    )?;

    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![coin(amount.u128(), denom)],
    });

    Ok(Response::new().add_message(transfer_msg))
}

pub fn repay(deps: DepsMut, info: MessageInfo) -> StdResult<Response> {
    let coin_sent = info.funds.first().unwrap();
    let debt_amount = load_debt_amount(deps.storage, &info.sender, &coin_sent.denom)?;

    DEBT_AMOUNT.save(
        deps.storage,
        (info.sender.clone(), coin_sent.denom.clone()),
        &debt_amount.checked_sub(coin_sent.amount)?,
    )?;

    Ok(Response::new())
}
