use cosmwasm_std::{DepsMut, Env};
use mars_owner::OwnerInit::SetInitialOwner;
use mars_types::credit_manager::InstantiateMsg;

use crate::{
    error::ContractResult,
    state::{
        HEALTH_CONTRACT, INCENTIVES, MAX_SLIPPAGE, MAX_UNLOCKING_POSITIONS, ORACLE, OWNER, PARAMS,
        RED_BANK, SWAPPER, ZAPPER,
    },
    utils::assert_max_slippage,
};

pub fn store_config(deps: DepsMut, env: Env, msg: &InstantiateMsg) -> ContractResult<()> {
    OWNER.initialize(
        deps.storage,
        deps.api,
        SetInitialOwner {
            owner: msg.owner.clone(),
        },
    )?;

    RED_BANK.save(deps.storage, &msg.red_bank.check(deps.api, env.contract.address.clone())?)?;
    ORACLE.save(deps.storage, &msg.oracle.check(deps.api)?)?;
    SWAPPER.save(deps.storage, &msg.swapper.check(deps.api)?)?;
    ZAPPER.save(deps.storage, &msg.zapper.check(deps.api)?)?;
    MAX_UNLOCKING_POSITIONS.save(deps.storage, &msg.max_unlocking_positions)?;

    assert_max_slippage(msg.max_slippage)?;
    MAX_SLIPPAGE.save(deps.storage, &msg.max_slippage)?;

    HEALTH_CONTRACT.save(deps.storage, &msg.health_contract.check(deps.api)?)?;
    PARAMS.save(deps.storage, &msg.params.check(deps.api)?)?;
    INCENTIVES.save(deps.storage, &msg.incentives.check(deps.api, env.contract.address)?)?;

    Ok(())
}
