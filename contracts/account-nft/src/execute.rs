use cosmwasm_std::{
    to_binary, DepsMut, Empty, Env, MessageInfo, QueryRequest, Response, WasmQuery,
};
use cw721::Cw721Execute;
use cw721_base::{
    ContractError::Ownership,
    OwnershipError::{NoOwner, NotOwner},
};
use mars_red_bank_types::oracle::ActionKind;
use mars_rover_health_types::{AccountKind, HealthValuesResponse, QueryMsg::HealthValues};

use crate::{
    contract::Parent,
    error::{
        ContractError,
        ContractError::{BaseError, BurnNotAllowed, HealthContractNotSet},
    },
    nft_config::NftConfigUpdates,
    state::{CONFIG, NEXT_ID},
};

pub fn mint(deps: DepsMut, info: MessageInfo, user: &str) -> Result<Response, ContractError> {
    let next_id = NEXT_ID.load(deps.storage)?;
    NEXT_ID.save(deps.storage, &(next_id + 1))?;
    Parent::default()
        .mint(deps, info, next_id.to_string(), user.to_string(), None, Empty {})
        .map_err(Into::into)
}

/// A few checks to ensure accounts are not accidentally deleted:
/// - Cannot burn if debt balance
/// - Cannot burn if collateral exceeding config set amount
pub fn burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let Some(health_contract_addr) = config.health_contract_addr else {
        return Err(HealthContractNotSet);
    };

    let response: HealthValuesResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: health_contract_addr.into(),
            msg: to_binary(&HealthValues {
                account_id: token_id.clone(),
                kind: AccountKind::Default,
                action: ActionKind::Default,
            })?,
        }))?;

    if !response.total_debt_value.is_zero() {
        return Err(BurnNotAllowed {
            reason: format!("Account has a debt balance. Value: {}.", response.total_debt_value),
        });
    }

    if response.total_collateral_value > config.max_value_for_burn {
        return Err(BurnNotAllowed {
            reason: format!(
                "Account collateral value exceeds config set max ({}). Total collateral value: {}.",
                config.max_value_for_burn, response.total_collateral_value
            ),
        });
    }

    Parent::default().burn(deps, env, info, token_id).map_err(Into::into)
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    updates: NftConfigUpdates,
) -> Result<Response, ContractError> {
    let current_minter =
        Parent::default().minter(deps.as_ref())?.minter.ok_or(BaseError(Ownership(NoOwner)))?;

    if info.sender != current_minter {
        return Err(BaseError(Ownership(NotOwner)));
    }

    let mut response = Response::new().add_attribute("action", "update_config");
    let mut config = CONFIG.load(deps.storage)?;

    if let Some(unchecked) = updates.health_contract_addr {
        let addr = deps.api.addr_validate(&unchecked)?;
        config.health_contract_addr = Some(addr.clone());
        response = response
            .add_attribute("key", "health_contract_addr")
            .add_attribute("value", addr.to_string());
    }

    if let Some(max) = updates.max_value_for_burn {
        config.max_value_for_burn = max;
        response = response
            .add_attribute("key", "max_value_for_burn")
            .add_attribute("value", max.to_string());
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(response)
}
