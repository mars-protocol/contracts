use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError,
};
use cw_vault_standard::{VaultInfoResponse, VaultStandardInfoResponse};
use mars_owner::OwnerInit;
use mars_types::{
    adapters::{account_nft::AccountNftBase, health::HealthContractBase, oracle::OracleBase},
    credit_manager::{ConfigResponse, QueryMsg as CreditManagerQueryMsg},
};

use crate::{
    base_vault::BaseVault,
    error::{ContractError, ContractResult},
    execute,
    msg::{
        ExecuteMsg, ExtensionExecuteMsg, ExtensionQueryMsg, InstantiateMsg, QueryMsg,
        VaultInfoResponseExt,
    },
    performance_fee::PerformanceFeeState,
    query,
    state::{
        ACCOUNT_NFT, COOLDOWN_PERIOD, CREDIT_MANAGER, DESCRIPTION, HEALTH, ORACLE, OWNER,
        PERFORMANCE_FEE_CONFIG, PERFORMANCE_FEE_STATE, SUBTITLE, TITLE, VAULT_ACC_ID,
    },
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

    let config: ConfigResponse = deps
        .querier
        .query_wasm_smart(credit_manager.to_string(), &CreditManagerQueryMsg::Config {})?;
    let oracle = OracleBase::new(config.oracle);
    let health = HealthContractBase::new(config.health_contract);
    ORACLE.save(deps.storage, &oracle.check(deps.api)?)?;
    HEALTH.save(deps.storage, &health.check(deps.api)?)?;
    if let Some(acc_nft) = config.account_nft {
        let account_nft = AccountNftBase::new(acc_nft);
        ACCOUNT_NFT.save(deps.storage, &account_nft.check(deps.api)?)?;
    } else {
        return Err(ContractError::Std(StdError::generic_err(
            "Account NFT contract address is not set in Credit Manager".to_string(),
        )));
    }

    if let Some(title) = msg.title {
        TITLE.save(deps.storage, &title)?;
    }
    if let Some(subtitle) = msg.subtitle {
        SUBTITLE.save(deps.storage, &subtitle)?;
    }
    if let Some(desc) = msg.description {
        DESCRIPTION.save(deps.storage, &desc)?;
    }

    COOLDOWN_PERIOD.save(deps.storage, &msg.cooldown_period)?;

    msg.performance_fee_config.validate()?;
    PERFORMANCE_FEE_CONFIG.save(deps.storage, &msg.performance_fee_config)?;
    PERFORMANCE_FEE_STATE.save(deps.storage, &PerformanceFeeState::default())?;

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
            ExtensionExecuteMsg::Unlock {
                amount,
            } => execute::unlock(deps, env, &info, amount),
            ExtensionExecuteMsg::WithdrawPerformanceFee {
                new_performance_fee_config,
            } => execute::withdraw_performance_fee(deps, env, &info, new_performance_fee_config),
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
                    vault_account_id: VAULT_ACC_ID.may_load(deps.storage)?,
                    cooldown_period: COOLDOWN_PERIOD.load(deps.storage)?,
                    performance_fee_config: PERFORMANCE_FEE_CONFIG.load(deps.storage)?,
                })
            }
            ExtensionQueryMsg::UserUnlocks {
                user_address,
            } => {
                let user_addr = deps.api.addr_validate(&user_address)?;
                to_json_binary(&query::unlocks(deps, user_addr)?)
            }
            ExtensionQueryMsg::PerformanceFeeState {} => {
                to_json_binary(&PERFORMANCE_FEE_STATE.load(deps.storage)?)
            }
        },
    }
    .map_err(Into::into)
}
