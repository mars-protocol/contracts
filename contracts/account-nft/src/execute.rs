use cosmwasm_std::{
    to_binary, DepsMut, Empty, Env, MessageInfo, QueryRequest, Response, WasmQuery,
};
use cw721::Cw721Execute;
use cw721_base::MintMsg;

use mars_health::HealthResponse;
use mars_rover::msg::QueryMsg::Health;

use crate::config::ConfigUpdates;
use crate::contract::Parent;
use crate::error::ContractError;
use crate::error::ContractError::{BaseError, BurnNotAllowed};
use crate::state::{CONFIG, NEXT_ID};

pub fn mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: &str,
) -> Result<Response, ContractError> {
    let next_id = NEXT_ID.load(deps.storage)?;
    let mint_msg_override = MintMsg {
        token_id: next_id.to_string(),
        owner: user.to_string(),
        token_uri: None,
        extension: Empty {},
    };
    NEXT_ID.save(deps.storage, &(next_id + 1))?;

    Parent::default()
        .mint(deps, env, info, mint_msg_override)
        .map_err(Into::into)
}

/// Checks first to ensure the balance of debts and collateral does not exceed the config
/// set amount. This is to ensure accounts are not accidentally deleted.
pub fn burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: String,
) -> Result<Response, ContractError> {
    let response: HealthResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        // Expects the minter to be the credit manager
        contract_addr: Parent::default().minter.load(deps.storage)?.into(),
        msg: to_binary(&Health {
            account_id: token_id.clone(),
        })?,
    }))?;

    let max_value_allowed = CONFIG.load(deps.storage)?.max_value_for_burn;
    let current_balances = response
        .total_debt_value
        .checked_add(response.total_collateral_value)?;
    if current_balances > max_value_allowed {
        return Err(BurnNotAllowed {
            current_balances,
            max_value_allowed,
        });
    }

    Parent::default()
        .burn(deps, env, info, token_id)
        .map_err(Into::into)
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    updates: ConfigUpdates,
) -> Result<Response, ContractError> {
    let current_minter = Parent::default().minter.load(deps.storage)?;
    if info.sender != current_minter {
        return Err(BaseError(cw721_base::ContractError::Unauthorized {}));
    }

    let mut response = Response::new().add_attribute("action", "rover/account_nft/update_config");
    let mut config = CONFIG.load(deps.storage)?;

    if let Some(max) = updates.max_value_for_burn {
        config.max_value_for_burn = max;
        response = response
            .add_attribute("key", "max_value_for_burn")
            .add_attribute("value", max.to_string());
    }

    if let Some(addr) = updates.proposed_new_minter {
        let validated = deps.api.addr_validate(&addr)?;
        config.proposed_new_minter = Some(validated);
        response = response
            .add_attribute("key", "pending_minter")
            .add_attribute("value", addr);
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(response)
}

pub fn accept_minter_role(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    let previous_minter = Parent::default().minter.load(deps.storage)?;

    match config.proposed_new_minter {
        Some(addr) if addr == info.sender => {
            Parent::default().minter.save(deps.storage, &addr)?;
            config.proposed_new_minter = None;
            CONFIG.save(deps.storage, &config)?;

            Ok(Response::new()
                .add_attribute("previous_minter", previous_minter)
                .add_attribute("new_minter", addr))
        }
        _ => Err(BaseError(cw721_base::ContractError::Unauthorized {})),
    }
}
