use cosmwasm_std::{BankMsg, Coin, CosmosMsg, DepsMut, MessageInfo, Response, StdResult, Uint128};

use crate::helpers::load_debt_amount;
use crate::state::DEBT_AMOUNT;

pub fn borrow(deps: DepsMut, info: MessageInfo, coin: Coin) -> StdResult<Response> {
    let debt_amount = load_debt_amount(deps.storage, &info.sender, &coin.denom)?;

    DEBT_AMOUNT.save(
        deps.storage,
        (info.sender.clone(), coin.denom.clone()),
        &debt_amount
            .checked_add(coin.amount)?
            .checked_add(Uint128::from(1u128))?, // The extra unit is simulated accrued interest
    )?;

    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![coin],
    });

    Ok(Response::new().add_message(transfer_msg))
}

pub fn repay(deps: DepsMut, info: MessageInfo, denom: String) -> StdResult<Response> {
    let debt_amount = load_debt_amount(deps.storage, &info.sender, &denom)?;
    let coin_sent = info.funds.first().unwrap();

    DEBT_AMOUNT.save(
        deps.storage,
        (info.sender.clone(), denom),
        &debt_amount.checked_sub(coin_sent.amount)?,
    )?;

    Ok(Response::new())
}
