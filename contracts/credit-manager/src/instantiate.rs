use std::collections::HashSet;

use cosmwasm_std::{Api, Decimal, DepsMut, QuerierWrapper, StdResult};
use mars_owner::OwnerInit::SetInitialOwner;
use mars_rover::{
    error::{ContractError::InvalidConfig, ContractResult},
    msg::{instantiate::VaultInstantiateConfig, InstantiateMsg},
};

use crate::state::{
    ALLOWED_COINS, MAX_CLOSE_FACTOR, MAX_UNLOCKING_POSITIONS, ORACLE, OWNER, RED_BANK, SWAPPER,
    VAULT_CONFIGS, ZAPPER,
};

pub fn store_config(deps: DepsMut, msg: &InstantiateMsg) -> ContractResult<()> {
    OWNER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.owner.clone(),
        },
    )?;

    RED_BANK.save(deps.storage, &msg.red_bank.check(deps.api)?)?;
    ORACLE.save(deps.storage, &msg.oracle.check(deps.api)?)?;
    SWAPPER.save(deps.storage, &msg.swapper.check(deps.api)?)?;
    ZAPPER.save(deps.storage, &msg.zapper.check(deps.api)?)?;
    MAX_UNLOCKING_POSITIONS.save(deps.storage, &msg.max_unlocking_positions)?;

    assert_lte_to_one(&msg.max_close_factor)?;
    MAX_CLOSE_FACTOR.save(deps.storage, &msg.max_close_factor)?;

    assert_no_duplicate_vaults(deps.api, &deps.querier, &msg.vault_configs)?;
    msg.vault_configs.iter().try_for_each(|v| -> ContractResult<_> {
        v.config.check()?;
        let vault = v.vault.check(deps.api)?;
        Ok(VAULT_CONFIGS.save(deps.storage, &vault.address, &v.config)?)
    })?;

    assert_no_duplicate_coins(&msg.allowed_coins)?;
    msg.allowed_coins
        .iter()
        .try_for_each(|denom| ALLOWED_COINS.insert(deps.storage, denom).map(|_| ()))?;

    Ok(())
}

pub fn assert_no_duplicate_vaults(
    api: &dyn Api,
    querier: &QuerierWrapper,
    vaults: &[VaultInstantiateConfig],
) -> ContractResult<()> {
    let set: HashSet<_> = vaults.iter().map(|v| v.vault.address.clone()).collect();
    if set.len() != vaults.len() {
        return Err(InvalidConfig {
            reason: "Duplicate vault configs present".to_string(),
        });
    }

    let set: HashSet<_> = vaults
        .iter()
        .map(|v| {
            let vault = v.vault.check(api)?;
            let info = vault.query_info(querier)?;
            Ok(info.vault_token)
        })
        .collect::<StdResult<_>>()?;
    if set.len() != vaults.len() {
        return Err(InvalidConfig {
            reason: "Multiple vaults share the same vault token".to_string(),
        });
    }

    Ok(())
}

pub fn assert_no_duplicate_coins(denoms: &[String]) -> ContractResult<()> {
    let set: HashSet<_> = denoms.iter().collect();
    if set.len() != denoms.len() {
        return Err(InvalidConfig {
            reason: "Duplicate coin configs present".to_string(),
        });
    }
    Ok(())
}

pub fn assert_lte_to_one(dec: &Decimal) -> ContractResult<()> {
    if dec > &Decimal::one() {
        return Err(InvalidConfig {
            reason: "value greater than one".to_string(),
        });
    }
    Ok(())
}
