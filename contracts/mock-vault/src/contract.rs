#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};

use rover::msg::vault::{ExecuteMsg, QueryMsg};

use crate::deposit::deposit;
use crate::error::ContractError;
use crate::msg::InstantiateMsg;
use crate::query::{
    query_coins_for_shares, query_unlocking_position, query_unlocking_positions,
    query_vault_coins_issued, query_vault_info,
};
use crate::state::{ASSETS, CHAIN_BANK, LOCKUP_TIME, LP_TOKEN_DENOM, NEXT_UNLOCK_ID, ORACLE};
use crate::unlock::{request_unlock, withdraw_unlocked, withdraw_unlocking_force};
use crate::withdraw::{withdraw, withdraw_force};

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
) -> StdResult<Response> {
    for denom in msg.asset_denoms {
        ASSETS.save(deps.storage, denom, &Uint128::zero())?;
    }
    LOCKUP_TIME.save(deps.storage, &msg.lockup)?;
    ORACLE.save(deps.storage, &msg.oracle.check(deps.api)?)?;
    LP_TOKEN_DENOM.save(deps.storage, &msg.lp_token_denom)?;
    CHAIN_BANK.save(deps.storage, &DEFAULT_VAULT_TOKEN_PREFUND)?;
    NEXT_UNLOCK_ID.save(deps.storage, &1)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {} => deposit(deps, info),
        ExecuteMsg::Withdraw {} => withdraw(deps, info),
        ExecuteMsg::ForceWithdraw {} => withdraw_force(deps, info),
        ExecuteMsg::ForceWithdrawUnlocking { lockup_id, amount } => {
            withdraw_unlocking_force(deps, &info.sender, lockup_id, amount)
        }
        ExecuteMsg::RequestUnlock {} => request_unlock(deps, env, info),
        ExecuteMsg::WithdrawUnlocked { id } => withdraw_unlocked(deps, env, &info.sender, id),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Info {} => to_binary(&query_vault_info(deps)?),
        QueryMsg::PreviewRedeem { amount } => {
            to_binary(&query_coins_for_shares(deps.storage, amount)?)
        }
        QueryMsg::TotalVaultCoinsIssued {} => to_binary(&query_vault_coins_issued(deps.storage)?),
        QueryMsg::UnlockingPositionsForAddr { addr } => {
            to_binary(&query_unlocking_positions(deps, addr)?)
        }
        QueryMsg::UnlockingPosition { id } => to_binary(&query_unlocking_position(deps, id)?),
    }
}
