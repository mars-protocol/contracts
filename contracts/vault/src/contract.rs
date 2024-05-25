use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response,
};
use cw_vault_standard::{VaultInfoResponse, VaultStandardInfoResponse};
use mars_owner::OwnerInit;

use crate::{
    base_vault::BaseVault,
    error::ContractResult,
    execute,
    msg::{
        ExecuteMsg, ExtensionExecuteMsg, ExtensionQueryMsg, InstantiateMsg, QueryMsg,
        VaultInfoResponseExt,
    },
    state::{CREDIT_MANAGER, CREDIT_MANAGER_ACC_ID, DESCRIPTION, OWNER, SUBTITLE, TITLE},
    token_factory::TokenFactoryDenom,
};

pub const CONTRACT_NAME: &str = "mars-vault";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const VAULT_STANDARD_VERSION: u16 = 1;

pub type Vault<'a> = BaseVault<'a>;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // initialize contract version info
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // initialize contract ownership info
    OWNER.initialize(
        deps.storage,
        deps.api,
        OwnerInit::SetInitialOwner {
            owner: info.sender.into(),
        },
    )?;

    let credit_manager = deps.api.addr_validate(&msg.credit_manager)?;
    CREDIT_MANAGER.save(deps.storage, &credit_manager.to_string())?;

    if let Some(tit) = msg.title {
        TITLE.save(deps.storage, &tit)?;
    }
    if let Some(subtitle) = msg.subtitle {
        SUBTITLE.save(deps.storage, &subtitle)?;
    }
    if let Some(desc) = msg.description {
        DESCRIPTION.save(deps.storage, &desc)?;
    }

    let base_vault = Vault::default();
    let vault_token =
        TokenFactoryDenom::new(env.contract.address.to_string(), msg.vault_token_subdenom);

    Ok(base_vault.init(deps, msg.base_token, vault_token)?)
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
        } => execute::deposit(deps, env, &info, recipient),
        ExecuteMsg::Redeem {
            recipient,
            amount: _, // don't care about amount, use funds data
        } => execute::redeem(deps, env, &info, recipient),
        ExecuteMsg::VaultExtension(msg) => match msg {
            ExtensionExecuteMsg::BindCreditManagerAccount {
                account_id,
            } => execute::bind_credit_manager_account(deps, &info, account_id),
        },
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    let base_vault = Vault::default();

    match msg {
        QueryMsg::VaultStandardInfo {} => to_json_binary(&VaultStandardInfoResponse {
            version: VAULT_STANDARD_VERSION,
            extensions: vec![],
        }),
        QueryMsg::Info {} => {
            let vault_token = base_vault.vault_token.load(deps.storage)?;
            let base_token = base_vault.base_token.load(deps.storage)?;

            to_json_binary(&VaultInfoResponse {
                base_token: base_token.to_string(),
                vault_token: vault_token.to_string(),
            })
        }
        QueryMsg::PreviewDeposit {
            amount,
        } => to_json_binary(&base_vault.query_simulate_deposit(deps, amount)?),
        QueryMsg::PreviewRedeem {
            amount,
        } => to_json_binary(&base_vault.query_simulate_withdraw(deps, amount)?),
        QueryMsg::TotalAssets {} => to_json_binary(&base_vault.query_total_assets(deps)?),
        QueryMsg::TotalVaultTokenSupply {} => {
            to_json_binary(&base_vault.query_total_vault_token_supply(deps)?)
        }
        QueryMsg::ConvertToShares {
            amount,
        } => to_json_binary(&base_vault.query_simulate_deposit(deps, amount)?),
        QueryMsg::ConvertToAssets {
            amount,
        } => to_json_binary(&base_vault.query_simulate_withdraw(deps, amount)?),
        QueryMsg::VaultExtension(msg) => match msg {
            ExtensionQueryMsg::VaultInfo => {
                let vault_token = base_vault.vault_token.load(deps.storage)?;
                let base_token = base_vault.base_token.load(deps.storage)?;
                to_json_binary(&VaultInfoResponseExt {
                    base_token: base_token.to_string(),
                    vault_token: vault_token.to_string(),
                    title: TITLE.may_load(deps.storage)?,
                    subtitle: SUBTITLE.may_load(deps.storage)?,
                    description: DESCRIPTION.may_load(deps.storage)?,
                    credit_manager: CREDIT_MANAGER.load(deps.storage)?,
                    fund_manager_account_id: CREDIT_MANAGER_ACC_ID.may_load(deps.storage)?,
                })
            }
        },
    }
    .map_err(Into::into)
}
