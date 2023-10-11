use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, WasmMsg};
use cw721_base::Action;
use mars_account_nft_types::{msg::ExecuteMsg as NftExecuteMsg, nft_config::NftConfigUpdates};
use mars_owner::OwnerUpdate;
use mars_rover::{
    adapters::rewards_collector::RewardsCollector, error::ContractResult,
    msg::instantiate::ConfigUpdates,
};
use mars_rover_health_types::AccountKind;

use crate::{
    execute::create_credit_account,
    state::{
        ACCOUNT_NFT, HEALTH_CONTRACT, INCENTIVES, MAX_SLIPPAGE, MAX_UNLOCKING_POSITIONS, ORACLE,
        OWNER, RED_BANK, REWARDS_COLLECTOR, SWAPPER, ZAPPER,
    },
    utils::assert_max_slippage,
};

pub fn update_config(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    updates: ConfigUpdates,
) -> ContractResult<Response> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let mut response = Response::new().add_attribute("action", "update_config");

    if let Some(unchecked) = updates.account_nft {
        let account_nft = unchecked.check(deps.api)?;
        ACCOUNT_NFT.save(deps.storage, &account_nft)?;

        // Accept ownership. NFT contract owner must have proposed Rover as a new owner first.
        let accept_ownership_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: account_nft.address().into(),
            funds: vec![],
            msg: to_binary(&NftExecuteMsg::UpdateOwnership(Action::AcceptOwnership))?,
        });

        response = response
            .add_message(accept_ownership_msg)
            .add_attribute("key", "account_nft")
            .add_attribute("value", unchecked.address());
    }

    if let Some(unchecked) = updates.oracle {
        ORACLE.save(deps.storage, &unchecked.check(deps.api)?)?;
        response =
            response.add_attribute("key", "oracle").add_attribute("value", unchecked.address());
    }

    if let Some(unchecked) = updates.red_bank {
        RED_BANK.save(deps.storage, &unchecked.check(deps.api, env.contract.address.clone())?)?;
        response =
            response.add_attribute("key", "red_bank").add_attribute("value", unchecked.address());
    }

    if let Some(unchecked) = updates.swapper {
        SWAPPER.save(deps.storage, &unchecked.check(deps.api)?)?;
        response =
            response.add_attribute("key", "swapper").add_attribute("value", unchecked.address());
    }

    if let Some(unchecked) = updates.zapper {
        ZAPPER.save(deps.storage, &unchecked.check(deps.api)?)?;
        response =
            response.add_attribute("key", "zapper").add_attribute("value", unchecked.address());
    }

    if let Some(num) = updates.max_unlocking_positions {
        MAX_UNLOCKING_POSITIONS.save(deps.storage, &num)?;
        response = response
            .add_attribute("key", "max_unlocking_positions")
            .add_attribute("value", num.to_string());
    }

    if let Some(num) = updates.max_slippage {
        assert_max_slippage(num)?;
        MAX_SLIPPAGE.save(deps.storage, &num)?;
        response =
            response.add_attribute("key", "max_slippage").add_attribute("value", num.to_string());
    }

    if let Some(unchecked) = updates.health_contract {
        HEALTH_CONTRACT.save(deps.storage, &unchecked.check(deps.api)?)?;
        response = response
            .add_attribute("key", "health_contract")
            .add_attribute("value", unchecked.address());
    }

    if let Some(unchecked) = updates.incentives {
        INCENTIVES.save(deps.storage, &unchecked.check(deps.api, env.contract.address)?)?;
        response =
            response.add_attribute("key", "incentives").add_attribute("value", unchecked.address());
    }

    if let Some(unchecked) = updates.rewards_collector {
        let rewards_collector_addr = deps.api.addr_validate(&unchecked)?;

        let account_nft = ACCOUNT_NFT.load(deps.storage)?;
        let next_id = account_nft.query_next_id(&deps.querier)?;
        REWARDS_COLLECTOR.save(
            deps.storage,
            &RewardsCollector {
                address: rewards_collector_addr.to_string(),
                account_id: next_id.clone(),
            },
        )?;

        let res = create_credit_account(deps, rewards_collector_addr, AccountKind::Default)?;

        response = response
            .add_submessages(res.messages)
            .add_attribute("key", "rewards_collector_account")
            .add_attribute("value", next_id);
    }

    Ok(response)
}

pub fn update_owner(
    deps: DepsMut,
    info: MessageInfo,
    update: OwnerUpdate,
) -> ContractResult<Response> {
    Ok(OWNER.update(deps, info, update)?)
}

pub fn update_nft_config(
    deps: DepsMut,
    info: MessageInfo,
    config: Option<NftConfigUpdates>,
    ownership: Option<Action>,
) -> ContractResult<Response> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let nft_contract = ACCOUNT_NFT.load(deps.storage)?;
    let mut response = Response::new();

    if let Some(updates) = config {
        let update_config_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: nft_contract.address().into(),
            funds: vec![],
            msg: to_binary(&NftExecuteMsg::UpdateConfig {
                updates,
            })?,
        });
        response =
            response.add_message(update_config_msg).add_attribute("action", "update_nft_config")
    }

    if let Some(action) = ownership {
        let update_ownership_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: nft_contract.address().into(),
            funds: vec![],
            msg: to_binary(&NftExecuteMsg::UpdateOwnership(action))?,
        });
        response =
            response.add_message(update_ownership_msg).add_attribute("action", "update_ownership")
    }

    Ok(response)
}
