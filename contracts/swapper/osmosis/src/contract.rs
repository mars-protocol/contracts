use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response};
use cw2::set_contract_version;
use mars_swapper_base::{ContractResult, SwapBase};
use mars_types::swapper::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::{config::OsmosisConfig, route::OsmosisRoute};

/// The Osmosis swapper contract inherits logic from the base swapper contract
pub type OsmosisSwap<'a> = SwapBase<'a, Empty, Empty, OsmosisRoute, OsmosisConfig>;

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<Response> {
    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;
    OsmosisSwap::default().instantiate(deps, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<OsmosisRoute, OsmosisConfig>,
) -> ContractResult<Response> {
    OsmosisSwap::default().execute(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    OsmosisSwap::default().query(deps, env, msg)
}
