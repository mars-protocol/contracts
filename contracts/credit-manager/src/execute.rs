use cosmwasm_std::{to_binary, Addr, CosmosMsg, DepsMut, Response, StdResult, WasmMsg};
use cw721_base::{ExecuteMsg, Extension, MintMsg};

use crate::state::CREDIT_ACCOUNT_NFT_CONTRACT;

pub fn try_create_credit_account(deps: DepsMut, user: Addr) -> StdResult<Response> {
    let contract_addr = CREDIT_ACCOUNT_NFT_CONTRACT.load(deps.storage)?;

    let nft_mint_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        funds: vec![],
        msg: to_binary(&ExecuteMsg::Mint(MintMsg::<Extension> {
            token_id: String::from("contract-will-generate"),
            owner: user.to_string(),
            token_uri: None,
            extension: None,
        }))?,
    });

    Ok(Response::new().add_message(nft_mint_msg).add_attribute("method", "CreateCreditAccount"))
}
