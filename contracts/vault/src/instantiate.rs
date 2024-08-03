use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use mars_owner::OwnerInit;
use mars_utils::helpers::validate_native_denom;

use crate::{
    error::{ContractError, ContractResult},
    msg::InstantiateMsg,
    performance_fee::PerformanceFeeState,
    state::{
        BASE_TOKEN, COOLDOWN_PERIOD, CREDIT_MANAGER, DESCRIPTION, OWNER, PERFORMANCE_FEE_CONFIG,
        PERFORMANCE_FEE_STATE, SUBTITLE, TITLE, VAULT_TOKEN,
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

    // save credit manager address
    let credit_manager = deps.api.addr_validate(&msg.credit_manager)?;
    CREDIT_MANAGER.save(deps.storage, &credit_manager.to_string())?;

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

    if msg.cooldown_period == 0 {
        return Err(ContractError::ZeroCooldownPeriod {});
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

    validate_native_denom(&msg.base_token)?;
    BASE_TOKEN.save(deps.storage, &msg.base_token)?;

    Ok(vault_token.instantiate()?)
}
