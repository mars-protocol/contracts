use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, DepsMut, Reply, Response, Uint128};

use crate::state::VAULT_REQUEST_TEMP_STORAGE;
use rover::adapters::{UpdateType, Vault, VaultPositionUpdate};
use rover::error::{ContractError, ContractResult};
use rover::extensions::AttrParse;

use crate::vault::utils::{assert_vault_is_whitelisted, update_vault_position};

#[cw_serde]
pub struct RequestTempStorage {
    pub account_id: String,
    pub amount: Uint128,
}

pub fn request_unlock_from_vault(
    deps: DepsMut,
    account_id: &str,
    vault: Vault,
    amount: Uint128,
) -> ContractResult<Response> {
    assert_vault_is_whitelisted(deps.storage, &vault)?;

    let vault_info = vault.query_info(&deps.querier)?;
    if vault_info.lockup.is_none() {
        return Err(ContractError::RequirementsNotMet(
            "This vault does not require lockup. Call withdraw directly.".to_string(),
        ));
    }

    VAULT_REQUEST_TEMP_STORAGE.save(
        deps.storage,
        &RequestTempStorage {
            account_id: account_id.to_string(),
            amount,
        },
    )?;

    let request_unlock_msg = vault.request_unlock_msg(&[Coin {
        denom: vault_info.token_denom,
        amount,
    }])?;

    Ok(Response::new()
        .add_submessage(request_unlock_msg)
        .add_attribute("action", "rover/credit_manager/vault/request_unlock"))
}

pub fn handle_unlock_request_reply(deps: DepsMut, reply: Reply) -> ContractResult<Response> {
    let RequestTempStorage { account_id, amount } =
        VAULT_REQUEST_TEMP_STORAGE.load(deps.storage)?;

    let unlock_event = reply.parse_unlock_event()?;
    let vault_addr = deps.api.addr_validate(unlock_event.vault_addr.as_str())?;

    update_vault_position(
        deps.storage,
        &account_id,
        &vault_addr,
        VaultPositionUpdate::Unlocking {
            id: unlock_event.id,
            amount,
            kind: UpdateType::Increment,
        },
    )?;

    update_vault_position(
        deps.storage,
        &account_id,
        &vault_addr,
        VaultPositionUpdate::Locked {
            amount,
            kind: UpdateType::Decrement,
        },
    )?;

    VAULT_REQUEST_TEMP_STORAGE.remove(deps.storage);

    Ok(Response::new().add_attribute(
        "action",
        "rover/credit_manager/vault/unlock_request/handle_reply",
    ))
}
