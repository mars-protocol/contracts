use cosmwasm_std::{
    Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, Event, MessageInfo, Response, StdResult, Uint128,
};
use cosmwasm_vault_standard::extensions::lockup::{
    UnlockingPosition, UNLOCKING_POSITION_ATTR_KEY, UNLOCKING_POSITION_CREATED_EVENT_TYPE,
};
use cw_utils::{Duration, Expiration};

use crate::error::ContractError;
use crate::state::{COIN_BALANCE, LOCKUP_TIME, NEXT_LOCKUP_ID, UNLOCKING_POSITIONS};
use crate::withdraw::{get_vault_token, withdraw_state_update};

pub fn request_unlock(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let lockup_time_opt = LOCKUP_TIME.load(deps.storage)?;
    let lockup_duration = lockup_time_opt.ok_or(ContractError::NotLockingVault {})?;

    let vault_token = get_vault_token(deps.storage, info.funds)?;
    let lock_amount = withdraw_state_update(deps.storage, vault_token.amount)?;

    let next_lockup_id = NEXT_LOCKUP_ID.load(deps.storage)?;

    let release_at = match lockup_duration {
        Duration::Height(h) => Expiration::AtHeight(env.block.height + h),
        Duration::Time(s) => Expiration::AtTime(env.block.time.plus_seconds(s)),
    };

    UNLOCKING_POSITIONS.update(deps.storage, info.sender.clone(), |opt| -> StdResult<_> {
        let mut unlocking_positions = opt.unwrap_or_default();
        unlocking_positions.push(UnlockingPosition {
            owner: info.sender.clone(),
            id: next_lockup_id,
            release_at,
            base_token_amount: lock_amount,
        });
        Ok(unlocking_positions)
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
    let lockups = UNLOCKING_POSITIONS
        .may_load(deps.storage, sender.clone())?
        .ok_or(ContractError::UnlockRequired {})?;

    let matching_position = lockups
        .iter()
        .find(|p| p.id == id)
        .ok_or(ContractError::UnlockRequired {})?
        .clone();

    if &matching_position.owner != sender {
        return Err(ContractError::Unauthorized {});
    }

    if !matching_position.release_at.is_expired(&env.block) {
        return Err(ContractError::UnlockNotReady {});
    }

    let remaining = lockups.into_iter().filter(|p| p.id != id).collect();
    UNLOCKING_POSITIONS.save(deps.storage, sender.clone(), &remaining)?;

    let underlying_coin = COIN_BALANCE.load(deps.storage)?;
    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: sender.to_string(),
        amount: vec![Coin {
            denom: underlying_coin.denom,
            amount: matching_position.base_token_amount,
        }],
    });
    Ok(Response::new().add_message(transfer_msg))
}

pub fn withdraw_unlocking_force(
    deps: DepsMut,
    sender: &Addr,
    lockup_id: u64,
    amounts: Option<Uint128>,
) -> Result<Response, ContractError> {
    let mut lockups = UNLOCKING_POSITIONS.load(deps.storage, sender.clone())?;
    let mut lockup = lockups
        .iter()
        .find(|p| p.id == lockup_id)
        .cloned()
        .ok_or(ContractError::LockupPositionNotFound(lockup_id))?;

    lockups.retain(|p| p.id != lockup_id);

    let amount_to_withdraw = match amounts {
        Some(a) => {
            lockup.base_token_amount -= a;
            lockups.push(lockup.clone());
            a
        }
        None => lockup.base_token_amount,
    };

    UNLOCKING_POSITIONS.save(deps.storage, sender.clone(), &lockups)?;

    let base_token = COIN_BALANCE.load(deps.storage)?;
    let transfer_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: sender.to_string(),
        amount: vec![Coin {
            denom: base_token.denom,
            amount: amount_to_withdraw,
        }],
    });
    Ok(Response::new().add_message(transfer_msg))
}
