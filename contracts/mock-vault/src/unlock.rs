use cosmwasm_std::{DepsMut, Env, Event, MessageInfo, Response, StdResult, Uint128};

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

    NEXT_UNLOCK_ID.save(deps.storage, &(next_unlock_id + Uint128::from(1u128)))?;

    let event = Event::new(UNLOCKING_POSITION_CREATED_EVENT_TYPE)
        .add_attribute(UNLOCKING_POSITION_ATTR, next_unlock_id);
    Ok(Response::new().add_event(event))
}

pub fn withdraw_unlocked(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: Uint128,
) -> Result<Response, ContractError> {
    let unlocking_positions = UNLOCKING_COINS
        .may_load(deps.storage, info.sender.clone())?
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
    UNLOCKING_COINS.save(deps.storage, info.sender.clone(), &remaining)?;

    _exchange(deps.storage, info.sender, matching_position.amount)
}
