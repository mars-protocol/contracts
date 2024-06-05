use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdError};
use mars_owner::OwnerInit;
use mars_types::{
    adapters::{account_nft::AccountNftBase, health::HealthContractBase, oracle::OracleBase},
    credit_manager::{ConfigResponse, QueryMsg as CreditManagerQueryMsg},
};

use crate::{
    error::{ContractError, ContractResult},
    msg::InstantiateMsg,
    performance_fee::PerformanceFeeState,
    state::{
        ACCOUNT_NFT, BASE_TOKEN, COOLDOWN_PERIOD, CREDIT_MANAGER, DESCRIPTION, HEALTH, ORACLE,
        OWNER, PERFORMANCE_FEE_CONFIG, PERFORMANCE_FEE_STATE, SUBTITLE, TITLE, VAULT_TOKEN,
    },
    token_factory::TokenFactoryDenom,
};

pub fn init(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    // initialize contract ownership info
    OWNER.initialize(
        deps.storage,
        deps.api,
        OwnerInit::SetInitialOwner {
            owner: info.sender.into(),
        },
    )?;

    // initialize addresses of external contracts
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

    // update contract metadata
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

    // initialize performance fee state
    msg.performance_fee_config.validate()?;
    PERFORMANCE_FEE_CONFIG.save(deps.storage, &msg.performance_fee_config)?;
    PERFORMANCE_FEE_STATE.save(deps.storage, &PerformanceFeeState::default())?;

    // initialize vault token
    let vault_token =
        TokenFactoryDenom::new(env.contract.address.to_string(), msg.vault_token_subdenom);
    VAULT_TOKEN.save(deps.storage, &vault_token)?;
    BASE_TOKEN.save(deps.storage, &msg.base_token)?;

    Ok(vault_token.instantiate()?)
}
