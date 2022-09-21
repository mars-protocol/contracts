use cosmwasm_std::{DepsMut, Empty, Env, Event, MessageInfo, Response};
use cw721_base::{ContractError, MintMsg};

use crate::contract::Parent;
use crate::state::{NEXT_ID, PENDING_OWNER};

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

    Parent::default().mint(deps, env, info, mint_msg_override)
}

pub fn propose_new_owner(
    deps: DepsMut,
    info: MessageInfo,
    new_owner: &str,
) -> Result<Response, ContractError> {
    let proposed_owner_addr = deps.api.addr_validate(new_owner)?;
    let current_owner = Parent::default().minter.load(deps.storage)?;

    if info.sender != current_owner {
        return Err(ContractError::Unauthorized {});
    }

    PENDING_OWNER.save(deps.storage, &proposed_owner_addr)?;

    Ok(Response::new().add_attribute("action", "rover/account_nft/propose_new_owner"))
}

pub fn accept_ownership(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let pending_owner = PENDING_OWNER.load(deps.storage)?;
    let previous_owner = Parent::default().minter.load(deps.storage)?;

    if info.sender != pending_owner {
        return Err(ContractError::Unauthorized {});
    }

    Parent::default()
        .minter
        .save(deps.storage, &pending_owner)?;

    PENDING_OWNER.remove(deps.storage);

    let event = Event::new("rover/account_nft/accept_ownership")
        .add_attribute("previous_owner", previous_owner)
        .add_attribute("new_owner", pending_owner);
    Ok(Response::new().add_event(event))
}
