use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, Empty, Env, MessageInfo, Response, StdResult, WasmMsg,
};
use cw721::OwnerOfResponse;
use cw721_base::QueryMsg;

use account_nft::msg::ExecuteMsg as NftExecuteMsg;
use rover::coins::Coins;
use rover::error::{ContractError, ContractResult};
use rover::msg::execute::{Action, CallbackMsg};
use rover::msg::instantiate::ConfigUpdates;

use crate::borrow::borrow;
use crate::deposit::deposit;
use crate::health::assert_below_max_ltv;
use crate::state::{ACCOUNT_NFT, ALLOWED_COINS, ALLOWED_VAULTS, ORACLE, OWNER, RED_BANK};

pub fn create_credit_account(deps: DepsMut, user: Addr) -> ContractResult<Response> {
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
    new_config: ConfigUpdates,
) -> ContractResult<Response> {
    let owner = OWNER.load(deps.storage)?;

    if info.sender != owner {
        return Err(ContractError::Unauthorized {
            user: info.sender.into(),
            action: "update config".to_string(),
        });
    }

    let mut response =
        Response::new().add_attribute("action", "rover/credit_manager/update_config");

    if let Some(addr_str) = new_config.account_nft {
        let validated = deps.api.addr_validate(&addr_str)?;
        ACCOUNT_NFT.save(deps.storage, &validated)?;

        // Accept ownership. NFT contract owner must have proposed Rover as a new owner first.
        let accept_ownership_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: addr_str.clone(),
            funds: vec![],
            msg: to_binary(&NftExecuteMsg::AcceptOwnership)?,
        });

        response = response
            .add_message(accept_ownership_msg)
            .add_attribute("key", "account_nft")
            .add_attribute("value", addr_str);
    }

    if let Some(addr_str) = new_config.owner {
        let validated = deps.api.addr_validate(&addr_str)?;
        OWNER.save(deps.storage, &validated)?;
        response = response
            .add_attribute("key", "owner")
            .add_attribute("value", addr_str);
    }

    if let Some(coins) = new_config.allowed_coins {
        coins
            .iter()
            .try_for_each(|denom| ALLOWED_COINS.save(deps.storage, denom, &Empty {}))?;
        response = response
            .add_attribute("key", "allowed_coins")
            .add_attribute("value", coins.join(", "));
    }

    if let Some(vaults) = new_config.allowed_vaults {
        vaults.iter().try_for_each(|unchecked| {
            let vault = deps.api.addr_validate(unchecked)?;
            ALLOWED_VAULTS.save(deps.storage, &vault, &Empty {})
        })?;
        response = response
            .add_attribute("key", "allowed_vaults")
            .add_attribute("value", vaults.join(", "));
    }

    if let Some(unchecked) = new_config.red_bank {
        RED_BANK.save(deps.storage, &unchecked.check(deps.api)?)?;
        response = response
            .add_attribute("key", "red_bank")
            .add_attribute("value", unchecked.address());
    }

    if let Some(unchecked) = new_config.oracle {
        ORACLE.save(deps.storage, &unchecked.check(deps.api)?)?;
        response = response
            .add_attribute("key", "oracle")
            .add_attribute("value", unchecked.address());
    }

    Ok(response)
}

pub fn dispatch_actions(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    token_id: &str,
    actions: &[Action],
) -> ContractResult<Response> {
    assert_is_token_owner(&deps, &info.sender, token_id)?;

    let mut response = Response::new();
    let mut callbacks: Vec<CallbackMsg> = vec![];
    let mut received_coins = Coins::from(info.funds.as_slice());

    for action in actions {
        match action {
            Action::Deposit(coin) => {
                response = deposit(deps.storage, response, token_id, coin, &mut received_coins)?;
            }
            Action::Borrow(coin) => callbacks.push(CallbackMsg::Borrow {
                token_id: token_id.to_string(),
                coin: coin.clone(),
            }),
        }
    }

    // after all deposits have been handled, we assert that the `received_natives` list is empty
    // this way, we ensure that the user does not send any extra fund which will get lost in the contract
    if !received_coins.is_empty() {
        return Err(ContractError::ExtraFundsReceived(received_coins));
    }

    // after user selected actions, we assert LTV is healthy; if not, throw error and revert all actions
    callbacks.extend([CallbackMsg::AssertBelowMaxLTV {
        token_id: token_id.to_string(),
    }]);

    let callback_msgs = callbacks
        .iter()
        .map(|callback| callback.into_cosmos_msg(&env.contract.address))
        .collect::<StdResult<Vec<CosmosMsg>>>()?;

    Ok(response
        .add_messages(callback_msgs)
        .add_attribute("action", "rover/execute/update_credit_account"))
}

pub fn execute_callback(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    callback: CallbackMsg,
) -> ContractResult<Response> {
    if info.sender != env.contract.address {
        return Err(ContractError::ExternalInvocation);
    }
    match callback {
        CallbackMsg::Borrow { coin, token_id } => borrow(deps, env, &token_id, coin),
        CallbackMsg::AssertBelowMaxLTV { token_id } => {
            assert_below_max_ltv(deps.as_ref(), env, &token_id)
        }
    }
}

pub fn assert_is_token_owner(deps: &DepsMut, user: &Addr, token_id: &str) -> ContractResult<()> {
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
