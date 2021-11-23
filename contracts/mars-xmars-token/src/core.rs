use cosmwasm_std::{Addr, Env, StdResult, Storage, Uint128};
use cw20_base::state::{BALANCES, TOKEN_INFO};
use cw20_base::ContractError;

use crate::snapshots::{capture_balance_snapshot, capture_total_supply_snapshot};

pub fn transfer(
    storage: &mut dyn Storage,
    env: &Env,
    option_sender: Option<&Addr>,
    option_recipient: Option<&Addr>,
    amount: Uint128,
) -> Result<(), ContractError> {
    if amount.is_zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    if let Some(sender_addr) = option_sender {
        let sender_balance_new = BALANCES.update(
            storage,
            sender_addr,
            |balance: Option<Uint128>| -> StdResult<_> {
                Ok(balance.unwrap_or_default().checked_sub(amount)?)
            },
        )?;
        capture_balance_snapshot(storage, env, sender_addr, sender_balance_new)?;
    };

    if let Some(recipient_addr) = option_recipient {
        let recipient_balance_new = BALANCES.update(
            storage,
            recipient_addr,
            |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
        )?;
        capture_balance_snapshot(storage, env, recipient_addr, recipient_balance_new)?;
    }

    Ok(())
}

pub fn burn(
    storage: &mut dyn Storage,
    env: &Env,
    sender_addr: &Addr,
    amount: Uint128,
) -> Result<(), ContractError> {
    // lower balance
    transfer(storage, env, Some(sender_addr), None, amount)?;

    // reduce total_supply
    let new_token_info = TOKEN_INFO.update(storage, |mut info| -> StdResult<_> {
        info.total_supply = info.total_supply.checked_sub(amount)?;
        Ok(info)
    })?;

    capture_total_supply_snapshot(storage, env, new_token_info.total_supply)?;
    Ok(())
}
