#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::{
        entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
    };
    use mars_red_bank_types::rewards_collector::{ExecuteMsg, InstantiateMsg, QueryMsg};
    use mars_rewards_collector_base::{contract::Collector, ContractResult};

    pub type OsmosisCollector<'a> = Collector<'a, Empty, Empty>;

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> ContractResult<Response> {
        let collector = OsmosisCollector::default();
        collector.instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        let collector = OsmosisCollector::default();
        collector.execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        let collector = OsmosisCollector::default();
        collector.query(deps, env, msg)
    }
}
