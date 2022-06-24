use cosmwasm_std::{
    to_binary, CosmosMsg, DepsMut, Env, Reply, Response, StdError, StdResult, SubMsg, SubMsgResult,
    WasmMsg,
};
use cw721_base::InstantiateMsg as NftInstantiateMsg;
use cw_utils::parse_instantiate_response_data;

use rover::InstantiateMsg;

use crate::state::{ALLOWED_ASSETS, ALLOWED_VAULTS, CREDIT_ACCOUNT_NFT_CONTRACT, OWNER};

pub const NFT_CONTRACT_INSTANTIATE_REPLY_ID: u64 = 1;

/// Rover credit accounts are NFTs
pub fn instantiate_nft_contract(code_id: u64, owner: String, env: Env) -> StdResult<SubMsg> {
    let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Instantiate {
        admin: Some(owner.clone()),
        code_id,
        msg: to_binary(&NftInstantiateMsg {
            name: String::from("Rover Credit Account"),
            symbol: String::from("RCA"),
            minter: env.contract.address.to_string(),
        })?,
        funds: vec![],
        label: "rover_credit_account_nft".to_string(),
    });
    Ok(SubMsg::reply_on_success(
        cosmos_msg,
        NFT_CONTRACT_INSTANTIATE_REPLY_ID,
    ))
}

/// After successful NFT account-nft instantiation, save the account-nft address
pub fn store_nft_contract_addr(deps: DepsMut, reply: Reply) -> StdResult<Response> {
    let contract_str = parse_reply_for_contract_addr(reply)?;
    let contract_addr = deps.api.addr_validate(&contract_str)?;
    CREDIT_ACCOUNT_NFT_CONTRACT.save(deps.storage, &contract_addr)?;
    Ok(Response::new())
}

fn parse_reply_for_contract_addr(reply: Reply) -> StdResult<String> {
    return match reply.result {
        SubMsgResult::Ok(res) => match res.data {
            None => Err(StdError::generic_err(
                "Submessage did not have data to parse",
            )),
            Some(data) => {
                let parsed = parse_instantiate_response_data(&data)
                    .map_err(|_| StdError::generic_err("Could not parse binary response data"))?;
                Ok(parsed.contract_address)
            }
        },
        SubMsgResult::Err(err) => Err(StdError::generic_err(err)),
    };
}

pub fn store_config(mut deps: DepsMut, msg: &InstantiateMsg) -> StdResult<()> {
    store_owner(&mut deps, msg)?;
    store_assets_and_vaults(&mut deps, msg)?;
    Ok(())
}

fn store_owner(deps: &mut DepsMut, msg: &InstantiateMsg) -> StdResult<()> {
    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.save(deps.storage, &owner)?;
    Ok(())
}

fn store_assets_and_vaults(deps: &mut DepsMut, msg: &InstantiateMsg) -> StdResult<()> {
    msg.allowed_vaults.iter().try_for_each(|vault| {
        ALLOWED_VAULTS.save(deps.storage, deps.api.addr_validate(vault)?, &true)
    })?;

    msg.allowed_assets.iter().try_for_each(|info| {
        ALLOWED_ASSETS.save(deps.storage, info.check(deps.api, None)?.into(), &true)
    })?;
    Ok(())
}
