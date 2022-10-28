use cosmwasm_std::{Addr, DepsMut, Env, Event, MessageInfo, Response, StdResult, Uint128};
use cw_utils::{Duration, Expiration};

use cosmos_vault_standard::extensions::lockup::{
    Lockup, UNLOCKING_POSITION_ATTR_KEY, UNLOCKING_POSITION_CREATED_EVENT_TYPE,
};

use crate::error::ContractError;
use crate::state::{LOCKUPS, LOCKUP_TIME, NEXT_LOCKUP_ID};
use crate::withdraw::{_exchange, get_vault_token};

pub fn request_unlock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let lockup_time_opt = LOCKUP_TIME.load(deps.storage)?;
    let lockup_duration = lockup_time_opt.ok_or(ContractError::NotLockingVault {})?;

    let vault_token = get_vault_token(deps.storage, info.funds)?;
    let next_lockup_id = NEXT_LOCKUP_ID.load(deps.storage)?;

    let release_at = match lockup_duration {
        Duration::Height(h) => Expiration::AtHeight(env.block.height + h),
        Duration::Time(s) => Expiration::AtTime(env.block.time.plus_seconds(s)),
    };

    LOCKUPS.update(deps.storage, info.sender.clone(), |opt| -> StdResult<_> {
        let mut lockups = opt.unwrap_or_default();
        lockups.push(Lockup {
            owner: info.sender.clone(),
            id: next_lockup_id,
            release_at,
            coin: vault_token,
        });
        Ok(lockups)
    })?;

    NEXT_LOCKUP_ID.save(deps.storage, &(next_lockup_id + 1))?;

    let event = Event::new(UNLOCKING_POSITION_CREATED_EVENT_TYPE)
        .add_attribute(UNLOCKING_POSITION_ATTR_KEY, next_lockup_id.to_string());
    Ok(Response::new().add_event(event))
}

pub fn withdraw_unlocked(
    deps: DepsMut,
    env: Env,
    sender: &Addr,
    id: u64,
) -> Result<Response, ContractError> {
    let unlocking_positions = LOCKUPS
        .may_load(deps.storage, sender.clone())?
        .ok_or(ContractError::UnlockRequired {})?;

    let matching_position = unlocking_positions
        .iter()
        .find(|p| p.id == id)
        .ok_or(ContractError::UnlockRequired {})?
        .clone();

    if !matching_position.release_at.is_expired(&env.block) {
        return Err(ContractError::UnlockNotReady {});
    }

    let remaining = unlocking_positions
        .into_iter()
        .filter(|p| p.id != id)
        .collect();
    LOCKUPS.save(deps.storage, sender.clone(), &remaining)?;

    _exchange(deps.storage, sender, matching_position.coin.amount)
}

pub fn withdraw_unlocking_force(
    deps: DepsMut,
    sender: &Addr,
    lockup_id: u64,
    amounts: Option<Uint128>,
) -> Result<Response, ContractError> {
    let mut lockups = LOCKUPS.load(deps.storage, sender.clone())?;
    let mut lockup = lockups
        .iter()
        .find(|p| p.id == lockup_id)
        .cloned()
        .ok_or(ContractError::LockupPositionNotFound(lockup_id))?;

    lockups.retain(|p| p.id != lockup_id);

    let amount_to_withdraw = match amounts {
        Some(a) => {
            lockup.coin.amount -= a;
            lockups.push(lockup.clone());
            a
        }
        None => lockup.coin.amount,
    };

    LOCKUPS.save(deps.storage, sender.clone(), &lockups)?;

    _exchange(deps.storage, sender, amount_to_withdraw)
}
