use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
};
use cw2::{get_contract_version, set_contract_version, ContractVersion};
use cw721_base::Cw721Contract;

use crate::{
    error::{ContractError, ContractError::MigrationError},
    execute::{burn, mint, update_config},
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    nft_config::NftConfig,
    query::{query_config, query_next_id},
    state::{CONFIG, NEXT_ID},
};

const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// Extending CW721 base contract
pub type Parent<'a> = Cw721Contract<'a, Empty, Empty, Empty, Empty>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    NEXT_ID.save(deps.storage, &1)?;

    let health_contract_addr = msg
        .health_contract
        .as_ref()
        .map(|unchecked| deps.api.addr_validate(unchecked))
        .transpose()?;

    CONFIG.save(
        deps.storage,
        &NftConfig {
            max_value_for_burn: msg.max_value_for_burn,
            health_contract_addr,
        },
    )?;

    Parent::default().instantiate(deps, env, info, msg.into())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Mint {
            user,
        } => mint(deps, info, &user),
        ExecuteMsg::UpdateConfig {
            updates,
        } => update_config(deps, info, updates),
        ExecuteMsg::Burn {
            token_id,
        } => burn(deps, env, info, token_id),
        _ => Parent::default().execute(deps, env, info, msg.try_into()?).map_err(Into::into),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::NextId {} => to_binary(&query_next_id(deps)?),
        _ => Parent::default().query(deps, env, msg.try_into()?),
    }
}

const FROM_VERSION: &str = "1.0.0";
const TO_VERSION: &str = "2.0.0";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
    let ContractVersion {
        contract,
        version,
    } = get_contract_version(deps.storage)?;

    if CONTRACT_NAME != contract {
        return Err(MigrationError {
            reason: format!("Wrong contract. Expected: {CONTRACT_NAME}, Found: {contract}"),
        });
    }

    if FROM_VERSION != version {
        return Err(MigrationError {
            reason: format!("Wrong version. Expected: {FROM_VERSION}, Found: {version}"),
        });
    }

    set_contract_version(deps.storage, CONTRACT_NAME, TO_VERSION)?;

    Ok(cw721_base::upgrades::v0_17::migrate::<Empty, Empty, Empty, Empty>(deps)?)
}
