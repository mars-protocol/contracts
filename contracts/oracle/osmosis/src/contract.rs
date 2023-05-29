use cosmwasm_std::Empty;
use mars_oracle_base::OracleBase;

use crate::price_source::{OsmosisPriceSourceChecked, OsmosisPriceSourceUnchecked};

/// The Osmosis oracle contract inherits logics from the base oracle contract, with the Osmosis query
/// and price source plugins
pub type OsmosisOracle<'a> =
    OracleBase<'a, OsmosisPriceSourceChecked, OsmosisPriceSourceUnchecked, Empty>;

pub const CONTRACT_NAME: &str = "crates.io:mars-oracle-osmosis";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Env, MessageInfo, Response};
    use mars_oracle_base::ContractResult;
    use mars_red_bank_types::oracle::{ExecuteMsg, InstantiateMsg, QueryMsg};

    use super::*;
    use crate::migrations;

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg,
    ) -> ContractResult<Response> {
        cw2::set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
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

    #[entry_point]
    pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> ContractResult<Response> {
        migrations::v1_0_0::migrate(deps)
    }
}
