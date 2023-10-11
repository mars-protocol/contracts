use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
};
use cw2::set_contract_version;
use mars_rover::{
    adapters::vault::VAULT_REQUEST_REPLY_ID,
    error::{ContractError, ContractResult},
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
};

use crate::{
    execute::{create_credit_account, dispatch_actions, execute_callback},
    instantiate::store_config,
    migrations,
    query::{
        query_accounts, query_all_coin_balances, query_all_debt_shares,
        query_all_total_debt_shares, query_all_vault_positions, query_config, query_positions,
        query_total_debt_shares, query_vault_position_value, query_vault_utilization,
    },
    repay::repay_from_wallet,
    update_config::{update_config, update_nft_config, update_owner},
    utils::get_account_kind,
    vault::handle_unlock_request_reply,
    zap::{estimate_provide_liquidity, estimate_withdraw_liquidity},
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;
    store_config(deps, env, &msg)?;
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
        ExecuteMsg::CreateCreditAccount(kind) => create_credit_account(deps, info.sender, kind),
        ExecuteMsg::UpdateConfig {
            updates,
        } => update_config(deps, env, info, updates),
        ExecuteMsg::UpdateNftConfig {
            config,
            ownership,
        } => update_nft_config(deps, info, config, ownership),
        ExecuteMsg::UpdateOwner(update) => update_owner(deps, info, update),
        ExecuteMsg::Callback(callback) => execute_callback(deps, info, env, callback),
        ExecuteMsg::UpdateCreditAccount {
            account_id,
            actions,
        } => dispatch_actions(deps, env, info, &account_id, actions),
        ExecuteMsg::RepayFromWallet {
            account_id,
        } => repay_from_wallet(deps, env, info, account_id),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _: Env, reply: Reply) -> ContractResult<Response> {
    match reply.id {
        VAULT_REQUEST_REPLY_ID => handle_unlock_request_reply(deps, reply),
        id => Err(ContractError::ReplyIdError(id)),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    let res = match msg {
        QueryMsg::AccountKind {
            account_id,
        } => to_binary(&get_account_kind(deps.storage, &account_id)?),
        QueryMsg::Accounts {
            owner,
            start_after,
            limit,
        } => to_binary(&query_accounts(deps, owner, start_after, limit)?),
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::VaultUtilization {
            vault,
        } => to_binary(&query_vault_utilization(deps, env, vault)?),
        QueryMsg::Positions {
            account_id,
        } => to_binary(&query_positions(deps, &account_id)?),
        QueryMsg::AllCoinBalances {
            start_after,
            limit,
        } => to_binary(&query_all_coin_balances(deps, start_after, limit)?),
        QueryMsg::AllDebtShares {
            start_after,
            limit,
        } => to_binary(&query_all_debt_shares(deps, start_after, limit)?),
        QueryMsg::TotalDebtShares(denom) => to_binary(&query_total_debt_shares(deps, &denom)?),
        QueryMsg::AllTotalDebtShares {
            start_after,
            limit,
        } => to_binary(&query_all_total_debt_shares(deps, start_after, limit)?),
        QueryMsg::AllVaultPositions {
            start_after,
            limit,
        } => to_binary(&query_all_vault_positions(deps, start_after, limit)?),
        QueryMsg::EstimateProvideLiquidity {
            lp_token_out,
            coins_in,
        } => to_binary(&estimate_provide_liquidity(deps, &lp_token_out, coins_in)?),
        QueryMsg::EstimateWithdrawLiquidity {
            lp_token,
        } => to_binary(&estimate_withdraw_liquidity(deps, lp_token)?),
        QueryMsg::VaultPositionValue {
            vault_position,
        } => to_binary(&query_vault_position_value(deps, vault_position)?),
    };
    res.map_err(Into::into)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> ContractResult<Response> {
    match msg {
        MigrateMsg::V1_0_0ToV2_0_0(updates) => migrations::v2_0_0::migrate(deps, env, updates),
    }
}
