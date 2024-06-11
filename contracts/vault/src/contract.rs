use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use cw_vault_standard::{VaultInfoResponse, VaultStandardInfoResponse};

use crate::{
    error::ContractResult,
    execute::{
        bind_credit_manager_account, deposit, redeem, total_base_tokens_in_account, unlock,
        withdraw_performance_fee,
    },
    instantiate::init,
    msg::{ExecuteMsg, ExtensionExecuteMsg, ExtensionQueryMsg, InstantiateMsg, QueryMsg},
    query::{
        convert_to_base_tokens, convert_to_vault_tokens, query_all_unlocks, query_user_unlocks,
        query_vault_info,
    },
    state::{BASE_TOKEN, PERFORMANCE_FEE_STATE, VAULT_TOKEN},
};

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const VAULT_STANDARD_VERSION: u16 = 1;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // initialize contract version info
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    init(deps, env, info, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Deposit {
            amount: _, // don't care about amount, use funds data
            recipient,
        } => deposit(deps, env, &info, recipient),
        ExecuteMsg::Redeem {
            recipient,
            amount: _, // don't care about amount, use funds data
        } => redeem(deps, env, &info, recipient),
        ExecuteMsg::VaultExtension(msg) => match msg {
            ExtensionExecuteMsg::BindCreditManagerAccount {
                account_id,
            } => bind_credit_manager_account(deps, &info, account_id),
            ExtensionExecuteMsg::Unlock {
                amount,
            } => unlock(deps, env, &info, amount),
            ExtensionExecuteMsg::WithdrawPerformanceFee {
                new_performance_fee_config,
            } => withdraw_performance_fee(deps, env, &info, new_performance_fee_config),
        },
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::VaultStandardInfo {} => to_json_binary(&VaultStandardInfoResponse {
            version: VAULT_STANDARD_VERSION,
            extensions: vec![],
        }),
        QueryMsg::Info {} => to_json_binary(&VaultInfoResponse {
            base_token: BASE_TOKEN.load(deps.storage)?,
            vault_token: VAULT_TOKEN.load(deps.storage)?.to_string(),
        }),
        QueryMsg::PreviewDeposit {
            amount,
        } => to_json_binary(&convert_to_vault_tokens(deps, amount)?),
        QueryMsg::PreviewRedeem {
            amount,
        } => to_json_binary(&convert_to_base_tokens(deps, amount)?),
        QueryMsg::TotalAssets {} => to_json_binary(&total_base_tokens_in_account(deps)?),
        QueryMsg::TotalVaultTokenSupply {} => {
            to_json_binary(&VAULT_TOKEN.load(deps.storage)?.query_total_supply(deps)?)
        }
        QueryMsg::ConvertToShares {
            amount,
        } => to_json_binary(&convert_to_vault_tokens(deps, amount)?),
        QueryMsg::ConvertToAssets {
            amount,
        } => to_json_binary(&convert_to_base_tokens(deps, amount)?),
        QueryMsg::VaultExtension(msg) => match msg {
            ExtensionQueryMsg::VaultInfo {} => to_json_binary(&query_vault_info(deps)?),
            ExtensionQueryMsg::UserUnlocks {
                user_address,
            } => {
                let user_addr = deps.api.addr_validate(&user_address)?;
                to_json_binary(&query_user_unlocks(deps, user_addr)?)
            }
            ExtensionQueryMsg::AllUnlocks {
                start_after,
                limit,
            } => to_json_binary(&query_all_unlocks(deps, start_after, limit)?),
            ExtensionQueryMsg::PerformanceFeeState {} => {
                to_json_binary(&PERFORMANCE_FEE_STATE.load(deps.storage)?)
            }
        },
    }
    .map_err(Into::into)
}
