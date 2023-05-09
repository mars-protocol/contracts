use cosmwasm_std::{to_binary, CosmosMsg, DepsMut, MessageInfo, Response, WasmMsg};
use cw721_base::Action;
use mars_account_nft::{msg::ExecuteMsg as NftExecuteMsg, nft_config::NftConfigUpdates};
use mars_owner::OwnerUpdate;
use mars_rover::{
    error::ContractResult,
    msg::instantiate::ConfigUpdates,
    traits::{FallbackStr, Stringify},
};

use crate::{
    instantiate::{assert_lte_to_one, assert_no_duplicate_coins, assert_no_duplicate_vaults},
    state::{
        ACCOUNT_NFT, ALLOWED_COINS, HEALTH_CONTRACT, MAX_CLOSE_FACTOR, MAX_UNLOCKING_POSITIONS,
        ORACLE, OWNER, RED_BANK, SWAPPER, VAULT_CONFIGS, ZAPPER,
    },
};

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    updates: ConfigUpdates,
) -> ContractResult<Response> {
    OWNER.assert_owner(deps.storage, &info.sender)?;

    let mut response = Response::new().add_attribute("action", "update_config");

    if let Some(addr_str) = updates.account_nft {
        let validated = deps.api.addr_validate(&addr_str)?;
        ACCOUNT_NFT.save(deps.storage, &validated)?;

        // Accept minter role. NFT contract minter must have proposed Rover as a new minter first.
        let accept_minter_role_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: addr_str.clone(),
            funds: vec![],
            msg: to_binary(&NftExecuteMsg::UpdateOwnership(Action::AcceptOwnership))?,
        });

        response = response
            .add_message(accept_minter_role_msg)
            .add_attribute("key", "account_nft")
            .add_attribute("value", addr_str);
    }

    if let Some(coins) = updates.allowed_coins {
        assert_no_duplicate_coins(&coins)?;
        ALLOWED_COINS.clear(deps.storage);
        coins.iter().try_for_each(|denom| ALLOWED_COINS.insert(deps.storage, denom).map(|_| ()))?;

        response = response
            .add_attribute("key", "allowed_coins")
            .add_attribute("value", coins.join(", ").fallback("None"));
    }

    if let Some(configs) = updates.vault_configs {
        assert_no_duplicate_vaults(deps.api, &deps.querier, &configs)?;
        VAULT_CONFIGS.clear(deps.storage);
        configs.iter().try_for_each(|v| -> ContractResult<_> {
            v.config.check()?;
            let vault = v.vault.check(deps.api)?;
            Ok(VAULT_CONFIGS.save(deps.storage, &vault.address, &v.config)?)
        })?;
        response = response
            .add_attribute("key", "vault_configs")
            .add_attribute("value", configs.to_string().fallback("None"))
    }

    if let Some(unchecked) = updates.oracle {
        ORACLE.save(deps.storage, &unchecked.check(deps.api)?)?;
        response =
            response.add_attribute("key", "oracle").add_attribute("value", unchecked.address());
    }

    if let Some(unchecked) = updates.red_bank {
        RED_BANK.save(deps.storage, &unchecked.check(deps.api)?)?;
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

    if let Some(cf) = updates.max_close_factor {
        assert_lte_to_one(&cf)?;
        MAX_CLOSE_FACTOR.save(deps.storage, &cf)?;
        response = response
            .add_attribute("key", "max_close_factor")
            .add_attribute("value", cf.to_string());
    }

    if let Some(num) = updates.max_unlocking_positions {
        MAX_UNLOCKING_POSITIONS.save(deps.storage, &num)?;
        response = response
            .add_attribute("key", "max_unlocking_positions")
            .add_attribute("value", num.to_string());
    }

    if let Some(unchecked) = updates.health_contract {
        HEALTH_CONTRACT.save(deps.storage, &unchecked.check(deps.api)?)?;
        response = response
            .add_attribute("key", "health_contract")
            .add_attribute("value", unchecked.address());
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
            contract_addr: nft_contract.to_string(),
            funds: vec![],
            msg: to_binary(&NftExecuteMsg::UpdateConfig {
                updates,
            })?,
        });
        response = response.add_message(update_config_msg).add_attribute("action", "update_config")
    }

    if let Some(action) = ownership {
        let update_ownership_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: nft_contract.to_string(),
            funds: vec![],
            msg: to_binary(&NftExecuteMsg::UpdateOwnership(action))?,
        });
        response =
            response.add_message(update_ownership_msg).add_attribute("action", "update_ownership")
    }

    Ok(response)
}
