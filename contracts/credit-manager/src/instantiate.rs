use cosmwasm_std::DepsMut;
use mars_owner::OwnerInit::SetInitialOwner;
use mars_rover::{error::ContractResult, msg::InstantiateMsg};

use crate::state::{
    HEALTH_CONTRACT, MAX_UNLOCKING_POSITIONS, ORACLE, OWNER, PARAMS, RED_BANK, SWAPPER, ZAPPER,
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
    HEALTH_CONTRACT.save(deps.storage, &msg.health_contract.check(deps.api)?)?;
    PARAMS.save(deps.storage, &msg.params.check(deps.api)?)?;

    Ok(())
}
