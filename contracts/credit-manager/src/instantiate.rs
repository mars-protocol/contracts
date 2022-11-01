use cosmwasm_std::DepsMut;

use rover::error::ContractResult;
use rover::msg::InstantiateMsg;

use crate::state::{
    ALLOWED_COINS, MAX_CLOSE_FACTOR, MAX_LIQUIDATION_BONUS, ORACLE, OWNER, RED_BANK, SWAPPER,
    VAULT_CONFIGS,
};

pub fn store_config(deps: DepsMut, msg: &InstantiateMsg) -> ContractResult<()> {
    let owner = deps.api.addr_validate(&msg.owner)?;
    OWNER.save(deps.storage, &owner)?;
    RED_BANK.save(deps.storage, &msg.red_bank.check(deps.api)?)?;
    ORACLE.save(deps.storage, &msg.oracle.check(deps.api)?)?;
    MAX_LIQUIDATION_BONUS.save(deps.storage, &msg.max_liquidation_bonus)?;
    MAX_CLOSE_FACTOR.save(deps.storage, &msg.max_close_factor)?;
    SWAPPER.save(deps.storage, &msg.swapper.check(deps.api)?)?;

    msg.allowed_vaults
        .iter()
        .try_for_each(|v| -> ContractResult<_> {
            v.config.check()?;
            let vault = v.vault.check(deps.api)?;
            Ok(VAULT_CONFIGS.save(deps.storage, &vault.address, &v.config)?)
        })?;

    msg.allowed_coins
        .iter()
        .try_for_each(|denom| ALLOWED_COINS.insert(deps.storage, denom).map(|_| ()))?;

    Ok(())
}
