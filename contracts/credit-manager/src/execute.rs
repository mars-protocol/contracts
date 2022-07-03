use cosmwasm_std::{
    to_binary, Addr, Attribute, CosmosMsg, DepsMut, MessageInfo, Response, StdError, StdResult,
    WasmMsg,
};
use account_nft::msg::{ExecuteMsg as NftExecuteMsg};

use crate::state::{ACCOUNT_NFT, OWNER};

pub fn try_create_credit_account(deps: DepsMut, user: Addr) -> StdResult<Response> {
    let contract_addr = ACCOUNT_NFT.load(deps.storage)?;

    let nft_mint_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        funds: vec![],
        msg: to_binary(&NftExecuteMsg::Mint { user: user.to_string() })?,
    });

    Ok(Response::new()
        .add_message(nft_mint_msg)
        .add_attribute("action", "rover/credit_manager/create_credit_account"))
}

pub fn try_update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_account_nft: Option<String>,
    new_owner: Option<String>,
) -> StdResult<Response> {
    let owner = OWNER.load(deps.storage)?;

    if info.sender != owner {
        return Err(StdError::generic_err(format!(
            "{} is not authorized to update config",
            info.sender
        )));
    }

    let mut attributes: Vec<Attribute> = vec![];

    if let Some(addr_str) = new_account_nft {
        let validated = deps.api.addr_validate(addr_str.as_str())?;
        ACCOUNT_NFT.save(deps.storage, &validated)?;
        attributes.push(Attribute::new(
            "action",
            "rover/credit_manager/update_config/account_nft",
        ));
    }

    if let Some(addr_str) = new_owner {
        let validated = deps.api.addr_validate(addr_str.as_str())?;
        OWNER.save(deps.storage, &validated)?;
        attributes.push(Attribute::new(
            "action",
            "rover/credit_manager/update_config/owner",
        ));
    }

    Ok(Response::new().add_attributes(attributes))
}
