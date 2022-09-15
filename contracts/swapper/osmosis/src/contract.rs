use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response};
use osmo_bindings::{OsmosisMsg, OsmosisQuery};

use rover::adapters::swap::{ExecuteMsg, InstantiateMsg, QueryMsg};
use swapper_base::{ContractResult, SwapBase};

use crate::route::OsmosisRoute;

/// The Osmosis swapper contract inherits logic from the base swapper contract
pub type OsmosisSwap<'a> = SwapBase<'a, OsmosisQuery, OsmosisMsg, OsmosisRoute>;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<OsmosisQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response<OsmosisMsg>> {
    OsmosisSwap::default().instantiate(deps, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<OsmosisQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<OsmosisRoute>,
) -> ContractResult<Response<OsmosisMsg>> {
    OsmosisSwap::default().execute(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<OsmosisQuery>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    OsmosisSwap::default().query(deps, env, msg)
}
