use cosmwasm_std::{entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use cw2::set_contract_version;

use rover::error::ContractResult;
use rover::msg::query::HealthResponse;
use rover::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::execute::{create_credit_account, dispatch_actions, execute_callback, update_config};
use crate::health::compute_health;
use crate::instantiate::store_config;
use crate::query::{
    query_all_coin_balances, query_all_debt_shares, query_all_total_debt_shares,
    query_all_total_vault_coin_balances, query_all_vault_positions, query_allowed_coins,
    query_allowed_vaults, query_config, query_position_with_value, query_total_debt_shares,
    query_total_vault_coin_balance,
};

const CONTRACT_NAME: &str = "crates.io:rover-credit-manager";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    store_config(deps, &msg)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::CreateCreditAccount {} => create_credit_account(deps, info.sender),
        ExecuteMsg::UpdateConfig { new_config } => update_config(deps, info, new_config),
        ExecuteMsg::Callback(callback) => execute_callback(deps, info, env, callback),
        ExecuteMsg::UpdateCreditAccount { token_id, actions } => {
            dispatch_actions(deps, env, info, &token_id, &actions)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    let res = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::AllowedVaults { start_after, limit } => {
            to_binary(&query_allowed_vaults(deps, start_after, limit)?)
        }
        QueryMsg::AllowedCoins { start_after, limit } => {
            to_binary(&query_allowed_coins(deps, start_after, limit)?)
        }
        QueryMsg::Positions { token_id } => {
            to_binary(&query_position_with_value(deps, &env, &token_id)?)
        }
        QueryMsg::Health { token_id } => {
            to_binary::<HealthResponse>(&Into::into(compute_health(deps, &env, &token_id)?))
        }
        QueryMsg::AllCoinBalances { start_after, limit } => {
            to_binary(&query_all_coin_balances(deps, start_after, limit)?)
        }
        QueryMsg::AllDebtShares { start_after, limit } => {
            to_binary(&query_all_debt_shares(deps, start_after, limit)?)
        }
        QueryMsg::TotalDebtShares(denom) => to_binary(&query_total_debt_shares(deps, &denom)?),
        QueryMsg::AllTotalDebtShares { start_after, limit } => {
            to_binary(&query_all_total_debt_shares(deps, start_after, limit)?)
        }
        QueryMsg::TotalVaultCoinBalance { vault } => to_binary(&query_total_vault_coin_balance(
            deps,
            &vault,
            &env.contract.address,
        )?),
        QueryMsg::AllTotalVaultCoinBalances { start_after, limit } => to_binary(
            &query_all_total_vault_coin_balances(deps, &env.contract.address, start_after, limit)?,
        ),
        QueryMsg::AllVaultPositions { start_after, limit } => {
            to_binary(&query_all_vault_positions(deps, start_after, limit)?)
        }
    };
    res.map_err(Into::into)
}
