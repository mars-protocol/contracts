use cosmwasm_std::{DepsMut, StdResult};
use rover::msg::InstantiateMsg;

use crate::state::{ALLOWED_COINS, ALLOWED_VAULTS, OWNER, RED_BANK};

pub fn store_config(deps: DepsMut, msg: &InstantiateMsg) -> StdResult<()> {
    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.save(deps.storage, &owner)?;

    RED_BANK.save(deps.storage, &msg.red_bank.check(deps.api)?)?;

    msg.allowed_vaults.iter().try_for_each(|vault| {
        ALLOWED_VAULTS.save(deps.storage, deps.api.addr_validate(vault)?, &true)
    })?;

    msg.allowed_coins
        .iter()
        .try_for_each(|denom| ALLOWED_COINS.save(deps.storage, denom, &true))?;

    Ok(())
}
