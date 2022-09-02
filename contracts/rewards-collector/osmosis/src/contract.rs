use mars_rewards_collector_base::CollectorBase;

use osmo_bindings::{OsmosisMsg, OsmosisQuery};

use crate::OsmosisRoute;

/// The Osmosis rewards collector contract inherits logics from the base oracle contract, with the
/// Osmosis custom msg, query, and swap route plugins
pub type OsmosisCollector<'a> = CollectorBase<'a, OsmosisRoute, OsmosisMsg, OsmosisQuery>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

    use mars_outpost::rewards_collector::{InstantiateMsg, QueryMsg};
    use mars_rewards_collector_base::ContractResult;

    use super::*;
    use crate::msg::ExecuteMsg;

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut<OsmosisQuery>,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg,
    ) -> ContractResult<Response> {
        OsmosisCollector::default().instantiate(deps, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut<OsmosisQuery>,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response<OsmosisMsg>> {
        OsmosisCollector::default().execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps<OsmosisQuery>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
        OsmosisCollector::default().query(deps, msg)
    }
}
