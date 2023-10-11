#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{coin, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, Uint128};
use cw_vault_standard::{
    extensions::{
        force_unlock::ForceUnlockExecuteMsg,
        lockup::{LockupExecuteMsg, LockupQueryMsg},
    },
    msg::{ExtensionExecuteMsg, ExtensionQueryMsg},
};
use mars_rover::adapters::vault::{ExecuteMsg, QueryMsg};

use crate::{
    deposit::deposit,
    error::ContractResult,
    msg::InstantiateMsg,
    query::{
        query_lockup_duration, query_unlocking_position, query_unlocking_positions,
        query_vault_info, query_vault_token_supply, shares_to_base_denom_amount,
    },
    state::{
        CHAIN_BANK, COIN_BALANCE, IS_EVIL, LOCKUP_TIME, NEXT_LOCKUP_ID, ORACLE, TOTAL_VAULT_SHARES,
        VAULT_TOKEN_DENOM,
    },
    unlock::{request_unlock, withdraw_unlocked, withdraw_unlocking_force},
    withdraw::{redeem_force, withdraw},
};

pub const STARTING_VAULT_SHARES: Uint128 = Uint128::new(1_000_000);

/// cw-multi-test does not yet have the ability to mint sdk coins. For this reason,
/// this contract expects to be pre-funded with vault tokens and it will simulate the mint.
pub const DEFAULT_VAULT_TOKEN_PREFUND: Uint128 = Uint128::new(1_000_000_000);

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    COIN_BALANCE.save(deps.storage, &coin(0, msg.base_token_denom))?;
    LOCKUP_TIME.save(deps.storage, &msg.lockup)?;
    ORACLE.save(deps.storage, &msg.oracle.check(deps.api)?)?;
    VAULT_TOKEN_DENOM.save(deps.storage, &msg.vault_token_denom)?;
    CHAIN_BANK.save(deps.storage, &DEFAULT_VAULT_TOKEN_PREFUND)?;
    NEXT_LOCKUP_ID.save(deps.storage, &1)?;
    TOTAL_VAULT_SHARES.save(deps.storage, &Uint128::zero())?;
    IS_EVIL.save(deps.storage, &msg.is_evil)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Deposit {
            ..
        } => deposit(deps, info),
        ExecuteMsg::Redeem {
            ..
        } => withdraw(deps, info),
        ExecuteMsg::VaultExtension(ext) => match ext {
            ExtensionExecuteMsg::Lockup(lockup_msg) => match lockup_msg {
                LockupExecuteMsg::WithdrawUnlocked {
                    lockup_id,
                    ..
                } => withdraw_unlocked(deps, env, &info.sender, lockup_id),
                LockupExecuteMsg::Unlock {
                    ..
                } => request_unlock(deps, env, info),
                LockupExecuteMsg::EmergencyUnlock {
                    ..
                } => unimplemented!(),
            },
            ExtensionExecuteMsg::ForceUnlock(force_msg) => match force_msg {
                ForceUnlockExecuteMsg::ForceRedeem {
                    ..
                } => redeem_force(deps, info),
                ForceUnlockExecuteMsg::ForceWithdrawUnlocking {
                    lockup_id,
                    amount,
                    ..
                } => withdraw_unlocking_force(deps, &info.sender, lockup_id, amount),
                _ => unimplemented!(),
            },
        },
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    let res = match msg {
        QueryMsg::TotalVaultTokenSupply {} => to_binary(&query_vault_token_supply(deps.storage)?),
        QueryMsg::Info {} => to_binary(&query_vault_info(deps)?),
        QueryMsg::PreviewRedeem {
            amount,
        } => to_binary(&shares_to_base_denom_amount(deps.storage, amount)?),
        QueryMsg::VaultExtension(ext) => match ext {
            ExtensionQueryMsg::Lockup(lockup_msg) => match lockup_msg {
                LockupQueryMsg::UnlockingPositions {
                    owner,
                    ..
                } => to_binary(&query_unlocking_positions(deps, owner)?),
                LockupQueryMsg::UnlockingPosition {
                    lockup_id,
                    ..
                } => to_binary(&query_unlocking_position(deps, lockup_id)?),
                LockupQueryMsg::LockupDuration {} => to_binary(&query_lockup_duration(deps)?),
            },
        },
        _ => unimplemented!(),
    };
    res.map_err(Into::into)
}
