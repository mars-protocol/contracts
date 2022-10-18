use cosmwasm_std::{Addr, DepsMut, Env, Event, MessageInfo, Response, StdResult, Uint128};

use rover::msg::vault::{
    UnlockingPosition, UNLOCKING_POSITION_ATTR, UNLOCKING_POSITION_CREATED_EVENT_TYPE,
};

use crate::error::ContractError;
use crate::state::{LOCKUP_TIME, NEXT_UNLOCK_ID, UNLOCKING_COINS};
use crate::withdraw::{_exchange, get_vault_token};

pub fn request_unlock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let lockup_time_opt = LOCKUP_TIME.load(deps.storage)?;
    let lockup_time = lockup_time_opt.ok_or(ContractError::NoLockupTime {})?;

    let vault_tokens = get_vault_token(deps.storage, info.funds)?;

    let next_unlock_id = NEXT_UNLOCK_ID.load(deps.storage)?;
    let unlocked_at = env.block.time.plus_seconds(lockup_time);
    UNLOCKING_COINS.update(deps.storage, info.sender, |opt| -> StdResult<_> {
        let mut unlocking_positions = opt.unwrap_or_default();
        unlocking_positions.push(UnlockingPosition {
            id: next_unlock_id,
            amount: vault_tokens.amount,
            unlocked_at,
        });
        Ok(unlocking_positions)
    })?;

    NEXT_UNLOCK_ID.save(deps.storage, &(next_unlock_id + 1))?;

    let event = Event::new(UNLOCKING_POSITION_CREATED_EVENT_TYPE)
        .add_attribute(UNLOCKING_POSITION_ATTR, next_unlock_id.to_string());
    Ok(Response::new().add_event(event))
}

pub fn withdraw_unlocked(
    deps: DepsMut,
    env: Env,
    sender: &Addr,
    id: u64,
) -> Result<Response, ContractError> {
    let unlocking_positions = UNLOCKING_COINS
        .may_load(deps.storage, sender.clone())?
        .ok_or(ContractError::UnlockRequired {})?;

    let matching_position = unlocking_positions
        .iter()
        .find(|p| p.id == id)
        .ok_or(ContractError::UnlockRequired {})?
        .clone();

    if matching_position.unlocked_at > env.block.time {
        return Err(ContractError::UnlockNotReady {});
    }

    let remaining = unlocking_positions
        .into_iter()
        .filter(|p| p.id != id)
        .collect();
    UNLOCKING_COINS.save(deps.storage, sender.clone(), &remaining)?;

    _exchange(deps.storage, sender, matching_position.amount)
}

pub fn withdraw_unlocking_force(
    deps: DepsMut,
    sender: &Addr,
    lockup_id: u64,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let mut unlocking_positions = UNLOCKING_COINS.load(deps.storage, sender.clone())?;
    let mut unlocking_position = unlocking_positions
        .iter()
        .find(|p| p.id == lockup_id)
        .cloned()
        .ok_or(ContractError::LockupPositionNotFound(lockup_id))?;

    unlocking_positions.retain(|p| p.id != lockup_id);

    let amount_to_withdraw = match amount {
        Some(a) if a != unlocking_position.amount => {
            unlocking_position.amount -= a;
            unlocking_positions.push(unlocking_position);
            a
        }
        _ => unlocking_position.amount,
    };

    UNLOCKING_COINS.save(deps.storage, sender.clone(), &unlocking_positions)?;

    _exchange(deps.storage, sender, amount_to_withdraw)
}
