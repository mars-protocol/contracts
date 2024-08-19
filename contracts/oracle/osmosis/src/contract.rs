use cosmwasm_std::Empty;
use mars_oracle_base::OracleBase;

use crate::{OsmosisPriceSourceChecked, OsmosisPriceSourceUnchecked};

/// The Osmosis oracle contract inherits logics from the base oracle contract, with the Osmosis query
/// and price source plugins
pub type OsmosisOracle<'a> =
    OracleBase<'a, OsmosisPriceSourceChecked, OsmosisPriceSourceUnchecked, Empty, Empty, Empty>;

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response};
    use cw2::set_contract_version;
    use mars_oracle_base::ContractResult;
    use mars_types::oracle::{ExecuteMsg, InstantiateMsg, QueryMsg};

    use super::*;

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg<Empty>,
    ) -> ContractResult<Response> {
        set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;
        OsmosisOracle::default().instantiate(deps, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<OsmosisPriceSourceUnchecked>,
    ) -> ContractResult<Response> {
        OsmosisOracle::default().execute(deps, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
        OsmosisOracle::default().query(deps, env, msg)
    }
}
