use cosmwasm_std::{DepsMut, Empty, Env, MessageInfo, Response};
use cw721_base::{ContractError, MintMsg};

use crate::contract::Parent;

pub fn try_mint(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    user: String,
) -> Result<Response, ContractError> {
    let parent = Parent::default();
    let num_tokens = parent.token_count(deps.storage)?;
    let mint_msg_override = MintMsg {
        token_id: (num_tokens + 1).to_string(),
        owner: user,
        token_uri: None,
        extension: Empty {},
    };
    parent.mint(deps, env, info, mint_msg_override)
}

pub fn try_update_owner(deps: DepsMut, new_owner: String) -> Result<Response, ContractError> {
    let validated_addr = deps.api.addr_validate(new_owner.as_str())?;
    Parent::default()
        .minter
        .save(deps.storage, &validated_addr)?;

    Ok(Response::new().add_attribute("action", "rover/account_nft/update_owner"))
}
