use cosmwasm_std::{to_binary, Addr, CosmosMsg, StdError, StdResult, Storage, Uint128, WasmMsg};

use cw20_base::state::{BALANCES, TOKEN_INFO};
use cw20_base::ContractError;

use crate::Config;

/// Deduct amount from sender balance and add it to recipient balance
/// Returns messages to be sent on the final response
pub fn transfer(
    storage: &mut dyn Storage,
    config: &Config,
    sender_address: Addr,
    recipient_address: Addr,
    amount: Uint128,
    finalize_on_red_bank: bool,
) -> Result<Vec<CosmosMsg>, ContractError> {
    if sender_address == recipient_address {
        return Err(StdError::generic_err("Sender and recipient cannot be the same").into());
    }

    if amount.is_zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let sender_previous_balance = decrease_balance(storage, &sender_address, amount)?;

    let recipient_previous_balance = increase_balance(storage, &recipient_address, amount)?;

    let total_supply = TOKEN_INFO.load(storage)?.total_supply;

    let mut messages = vec![];

    // If the transfer results from a method called on the money market,
    // it is finalized there. Else it needs to update state and perform some validations
    // to ensure the transfer can be executed
    if finalize_on_red_bank {
        messages.push(finalize_transfer_msg(
            config.red_bank_address.clone(),
            sender_address.clone(),
            recipient_address.clone(),
            sender_previous_balance,
            recipient_previous_balance,
            amount,
        )?);
    }

    // Build incentives messagess
    messages.push(balance_change_msg(
        config.incentives_address.clone(),
        sender_address,
        sender_previous_balance,
        total_supply,
    )?);
    messages.push(balance_change_msg(
        config.incentives_address.clone(),
        recipient_address,
        recipient_previous_balance,
        total_supply,
    )?);

    Ok(messages)
}

/// Lower user balance and commit to store, returns previous balance
pub fn decrease_balance(
    storage: &mut dyn Storage,
    address: &Addr,
    amount: Uint128,
) -> Result<Uint128, StdError> {
    let previous_balance = BALANCES.load(storage, address).unwrap_or_default();
    let new_balance = previous_balance.checked_sub(amount)?;
    BALANCES.save(storage, address, &new_balance)?;

    Ok(previous_balance)
}

/// Increase user balance and commit to store, returns previous balance
pub fn increase_balance(
    storage: &mut dyn Storage,
    address: &Addr,
    amount: Uint128,
) -> Result<Uint128, StdError> {
    let previous_balance = BALANCES.load(storage, address).unwrap_or_default();
    let new_balance = previous_balance + amount;
    BALANCES.save(storage, address, &new_balance)?;

    Ok(previous_balance)
}

pub fn finalize_transfer_msg(
    red_bank_address: Addr,
    sender_address: Addr,
    recipient_address: Addr,
    sender_previous_balance: Uint128,
    recipient_previous_balance: Uint128,
    amount: Uint128,
) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: red_bank_address.into(),
        msg: to_binary(
            &mars_core::red_bank::msg::ExecuteMsg::FinalizeLiquidityTokenTransfer {
                sender_address,
                recipient_address,
                sender_previous_balance,
                recipient_previous_balance,
                amount,
            },
        )?,
        funds: vec![],
    }))
}

pub fn balance_change_msg(
    incentives_address: Addr,
    user_address: Addr,
    user_balance_before: Uint128,
    total_supply_before: Uint128,
) -> StdResult<CosmosMsg> {
    Ok(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: incentives_address.into(),
        msg: to_binary(&mars_core::incentives::msg::ExecuteMsg::BalanceChange {
            user_address,
            user_balance_before,
            total_supply_before,
        })?,
        funds: vec![],
    }))
}
