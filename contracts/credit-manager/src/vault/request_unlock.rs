use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, DepsMut, Reply, Response, Uint128};

use crate::state::VAULT_REQUEST_TEMP_STORAGE;
use mars_rover::adapters::vault::{
    UnlockingChange, UpdateType, Vault, VaultBase, VaultPositionUpdate, VaultUnlockingPosition,
};
use mars_rover::error::{ContractError, ContractResult};
use mars_rover::extensions::AttrParse;

use crate::vault::utils::{assert_vault_is_whitelisted, update_vault_position};

#[cw_serde]
pub struct RequestTempStorage {
    pub account_id: String,
    pub amount: Uint128,
    pub vault_addr: Addr,
}

pub fn request_vault_unlock(
    deps: DepsMut,
    account_id: &str,
    vault: Vault,
    amount: Uint128,
) -> ContractResult<Response> {
    assert_vault_is_whitelisted(deps.storage, &vault)?;

    vault.query_lockup_duration(&deps.querier).map_err(|_| {
        ContractError::RequirementsNotMet(
            "This vault does not require lockup. Call withdraw directly.".to_string(),
        )
    })?;

    update_vault_position(
        deps.storage,
        account_id,
        &vault.address,
        VaultPositionUpdate::Locked(UpdateType::Decrement(amount)),
    )?;

    VAULT_REQUEST_TEMP_STORAGE.save(
        deps.storage,
        &RequestTempStorage {
            account_id: account_id.to_string(),
            amount,
            vault_addr: vault.address.clone(),
        },
    )?;

    let vault_info = vault.query_info(&deps.querier)?;
    let request_unlock_msg = vault.request_unlock_msg(Coin {
        denom: vault_info.vault_token,
        amount,
    })?;

    Ok(Response::new()
        .add_submessage(request_unlock_msg)
        .add_attribute("action", "rover/credit-manager/vault/request_unlock"))
}

pub fn handle_unlock_request_reply(deps: DepsMut, reply: Reply) -> ContractResult<Response> {
    let storage = VAULT_REQUEST_TEMP_STORAGE.load(deps.storage)?;
    let unlock_event = reply.parse_unlock_event()?;
    let vault = VaultBase::new(storage.vault_addr.clone());
    let unlocking_position = vault.query_unlocking_position(&deps.querier, unlock_event.id)?;
    let info = vault.query_info(&deps.querier)?;

    update_vault_position(
        deps.storage,
        &storage.account_id,
        &storage.vault_addr,
        VaultPositionUpdate::Unlocking(UnlockingChange::Add(VaultUnlockingPosition {
            id: unlocking_position.id,
            coin: Coin {
                denom: info.base_token,
                amount: unlocking_position.base_token_amount,
            },
        })),
    )?;

    VAULT_REQUEST_TEMP_STORAGE.remove(deps.storage);

    Ok(Response::new().add_attribute(
        "action",
        "rover/credit-manager/vault/unlock_request/handle_reply",
    ))
}
