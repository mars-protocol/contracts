use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response};

use rover::adapters::swap::{ExecuteMsg, InstantiateMsg, QueryMsg};
use swapper_base::{ContractResult, SwapBase};

use crate::route::OsmosisRoute;

/// The Osmosis swapper contract inherits logic from the base swapper contract
pub type OsmosisSwap<'a> = SwapBase<'a, Empty, Empty, OsmosisRoute>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    OsmosisSwap::default().instantiate(deps, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<OsmosisRoute>,
) -> ContractResult<Response> {
    OsmosisSwap::default().execute(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    OsmosisSwap::default().query(deps, env, msg)
}
