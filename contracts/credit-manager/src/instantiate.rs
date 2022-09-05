use cosmwasm_std::{DepsMut, Empty, StdResult};
use rover::msg::InstantiateMsg;

use crate::state::{ALLOWED_COINS, ALLOWED_VAULTS, ORACLE, OWNER, RED_BANK};

pub fn store_config(deps: DepsMut, msg: &InstantiateMsg) -> StdResult<()> {
    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.save(deps.storage, &owner)?;
    RED_BANK.save(deps.storage, &msg.red_bank.check(deps.api)?)?;
    ORACLE.save(deps.storage, &msg.oracle.check(deps.api)?)?;

    msg.allowed_vaults.iter().try_for_each(|unchecked| {
        let vault = unchecked.check(deps.api)?;
        ALLOWED_VAULTS.save(deps.storage, vault.address(), &Empty {})
    })?;

    msg.allowed_coins
        .iter()
        .try_for_each(|denom| ALLOWED_COINS.save(deps.storage, denom, &Empty {}))?;

    Ok(())
}
