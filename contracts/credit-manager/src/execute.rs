use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdResult, WasmMsg,
};
use cw721::OwnerOfResponse;
use cw721_base::QueryMsg;
use cw_asset::AssetList;

use account_nft::msg::ExecuteMsg as NftExecuteMsg;
use rover::msg::execute::{Action, CallbackMsg};

use crate::deposit::native_deposit;
use crate::error::ContractError;

use crate::state::{ACCOUNT_NFT, OWNER};

pub fn create_credit_account(deps: DepsMut, user: Addr) -> Result<Response, ContractError> {
    let contract_addr = ACCOUNT_NFT.load(deps.storage)?;

    let nft_mint_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: contract_addr.to_string(),
        funds: vec![],
        msg: to_binary(&NftExecuteMsg::Mint {
            user: user.to_string(),
        })?,
    });

    Ok(Response::new()
        .add_message(nft_mint_msg)
        .add_attribute("action", "rover/credit_manager/create_credit_account"))
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    new_account_nft: Option<String>,
    new_owner: Option<String>,
) -> Result<Response, ContractError> {
    let owner = OWNER.load(deps.storage)?;

    if info.sender != owner {
        return Err(ContractError::Unauthorized {
            user: info.sender.into(),
            action: "update config".to_string(),
        });
    }

    let mut response = Response::new();

    if let Some(addr_str) = new_account_nft {
        let validated = deps.api.addr_validate(&addr_str)?;
        ACCOUNT_NFT.save(deps.storage, &validated)?;

        // Accept ownership. NFT contract owner must have proposed Rover as a new owner first.
        let accept_ownership_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: addr_str,
            funds: vec![],
            msg: to_binary(&NftExecuteMsg::AcceptOwnership {})?,
        });

        response = response
            .add_message(accept_ownership_msg)
            .add_attribute("action", "rover/credit_manager/update_config/account_nft");
    }

    if let Some(addr_str) = new_owner {
        let validated = deps.api.addr_validate(&addr_str)?;
        OWNER.save(deps.storage, &validated)?;
        response = response.add_attribute("action", "rover/credit_manager/update_config/owner");
    }

    Ok(response)
}

pub fn dispatch_actions(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: &str,
    actions: &[Action],
) -> Result<Response, ContractError> {
    assert_is_token_owner(&deps, &info.sender, token_id)?;

    let mut response = Response::new();
    let mut callbacks: Vec<CallbackMsg> = vec![];
    let mut received_coins = AssetList::from(&info.funds);

    for action in actions {
        match action {
            Action::NativeDeposit(asset) => {
                response = native_deposit(
                    deps.storage,
                    deps.api,
                    response,
                    token_id,
                    asset,
                    &mut received_coins,
                )?;
            }
            Action::Placeholder { .. } => callbacks.push(CallbackMsg::Placeholder {}),
        }
    }

    // after all deposits have been handled, we assert that the `received_natives` list is empty
    // this way, we ensure that the user does not send any extra fund which will get lost in the contract
    if !received_coins.is_empty() {
        return Err(ContractError::ExtraFundsReceived(received_coins));
    }

    let callback_msgs = callbacks
        .iter()
        .map(|callback| callback.into_cosmos_msg(&env.contract.address))
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    Ok(response
        .add_messages(callback_msgs)
        .add_attribute("action", "rover/execute/update_credit_account"))
}

pub fn execute_callback(
    _deps: DepsMut,
    info: MessageInfo,
    env: Env,
    callback: CallbackMsg,
) -> Result<Response, ContractError> {
    if info.sender != env.contract.address {
        return Err(ContractError::ExternalInvocation {});
    }
    match callback {
        CallbackMsg::Placeholder { .. } => Ok(Response::new()),
    }
}

pub fn assert_is_token_owner(
    deps: &DepsMut,
    user: &Addr,
    token_id: &str,
) -> Result<(), ContractError> {
    let contract_addr = ACCOUNT_NFT.load(deps.storage)?;
    let owner_res: OwnerOfResponse = deps.querier.query_wasm_smart(
        contract_addr,
        &QueryMsg::OwnerOf {
            token_id: token_id.to_string(),
            include_expired: None,
        },
    )?;

    if user != &owner_res.owner {
        return Err(ContractError::NotTokenOwner {
            user: user.to_string(),
            token_id: token_id.to_string(),
        });
    }

    Ok(())
}
