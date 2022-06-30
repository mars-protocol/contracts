use cosmwasm_std::{DepsMut, StdResult};

use rover::InstantiateMsg;

use crate::state::{ACCOUNT_NFT, ALLOWED_ASSETS, ALLOWED_VAULTS, OWNER};

pub fn store_config(deps: DepsMut, msg: &InstantiateMsg) -> StdResult<()> {
    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.save(deps.storage, &owner)?;

    ACCOUNT_NFT.save(deps.storage, &None)?;

    msg.allowed_vaults.iter().try_for_each(|vault| {
        ALLOWED_VAULTS.save(deps.storage, deps.api.addr_validate(vault)?, &true)
    })?;

    msg.allowed_assets.iter().try_for_each(|info| {
        ALLOWED_ASSETS.save(deps.storage, info.check(deps.api, None)?.into(), &true)
    })?;
    Ok(())
}
