use cosmwasm_std::Empty;
use mars_rewards_collector_base::CollectorBase;

use crate::OsmosisRoute;

/// The Osmosis rewards collector contract inherits logics from the base oracle contract, with the
/// Osmosis custom msg, query, and swap route plugins
pub type OsmosisCollector<'a> = CollectorBase<'a, OsmosisRoute, Empty, Empty>;

pub const CONTRACT_NAME: &str = "crates.io:mars-rewards-collector-osmosis";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
    use mars_red_bank_types::rewards_collector::{InstantiateMsg, QueryMsg};
    use mars_rewards_collector_base::ContractResult;

    use super::*;
    use crate::msg::ExecuteMsg;

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg,
    ) -> ContractResult<Response> {
        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
        OsmosisCollector::default().instantiate(deps, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        OsmosisCollector::default().execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
        OsmosisCollector::default().query(deps, msg)
    }
}
