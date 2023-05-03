use cosmwasm_std::{
    entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};
use mars_v3_zapper_base::{
    contract::V3ZapperBase,
    error::ContractResult,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

use crate::position_manager::OsmosisPositionManager;

/// The Osmosis v3 zapper contract inherits logic from the base v3 zapper contract
pub type OsmosisV3Zapper = V3ZapperBase<OsmosisPositionManager>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    OsmosisV3Zapper::default().instantiate(deps, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<Response> {
    OsmosisV3Zapper::default().execute(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    OsmosisV3Zapper::default().query(deps, env, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> ContractResult<Response> {
    OsmosisV3Zapper::default().reply(deps, env, reply)
}
