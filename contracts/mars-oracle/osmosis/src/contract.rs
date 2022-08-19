use mars_oracle_base::OracleBase;

use osmo_bindings::OsmosisQuery;

use crate::OsmosisPriceSource;

/// The Osmosis oracle contract inherits logics from the base oracle contract, with the Osmosis query
/// and price source plugins
pub type OsmosisOracle<'a> = OracleBase<'a, OsmosisPriceSource, OsmosisQuery>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

    use mars_oracle_base::ContractResult;
    use mars_outpost::oracle::{ExecuteMsg, InstantiateMsg, QueryMsg};

    use super::*;

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut<OsmosisQuery>,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg,
    ) -> StdResult<Response> {
        OsmosisOracle::default().instantiate(deps, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut<OsmosisQuery>,
        _env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<OsmosisPriceSource>,
    ) -> ContractResult<Response> {
        OsmosisOracle::default().execute(deps, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps<OsmosisQuery>, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        OsmosisOracle::default().query(deps, env, msg)
    }
}
