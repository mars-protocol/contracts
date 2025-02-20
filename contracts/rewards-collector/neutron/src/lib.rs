pub mod migrations;

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::{
        entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
    };
    use cw2::set_contract_version;
    use mars_rewards_collector_base::{contract::Collector, ContractResult};
    use mars_types::rewards_collector::{ExecuteMsg, InstantiateMsg, QueryMsg};

    use crate::migrations;

    pub type NeutronCollector<'a> = Collector<'a, Empty, Empty>;

    pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
    pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> ContractResult<Response> {
        set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;
        let collector = NeutronCollector::default();
        collector.instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        let collector = NeutronCollector::default();
        collector.execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        let collector = NeutronCollector::default();
        collector.query(deps, env, msg)
    }

    #[entry_point]
    pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> ContractResult<Response> {
            migrations::v2_0_0::migrate(deps)
    }
}
