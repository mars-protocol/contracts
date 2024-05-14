use cosmwasm_std::Empty;
use mars_oracle_base::OracleBase;
use mars_types::oracle::{WasmOracleCustomExecuteMsg, WasmOracleCustomInitParams};

use crate::{
    astroport_twap::ExecuteTwapSnapshots, WasmPriceSourceChecked, WasmPriceSourceUnchecked,
};

/// The Wasm oracle contract inherits logics from the base oracle contract, with the Wasm query
/// and price source plugins
pub type WasmOracle<'a> = OracleBase<
    'a,
    WasmPriceSourceChecked,
    WasmPriceSourceUnchecked,
    Empty,
    WasmOracleCustomInitParams,
    WasmOracleCustomExecuteMsg,
>;

pub const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::{entry_point, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response};
    use cw2::set_contract_version;
    use mars_oracle_base::{ContractError, ContractResult};
    use mars_types::oracle::{ExecuteMsg, InstantiateMsg, QueryMsg};

    use super::*;
    use crate::{migrations, state::ASTROPORT_FACTORY, WasmPriceSourceUnchecked};

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        msg: InstantiateMsg<WasmOracleCustomInitParams>,
    ) -> ContractResult<Response> {
        set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

        let custom_init =
            msg.custom_init.as_ref().ok_or(ContractError::MissingCustomInitParams {})?;

        let astroport_factory = deps.api.addr_validate(&custom_init.astroport_factory)?;
        ASTROPORT_FACTORY.save(deps.storage, &astroport_factory)?;

        let contract = WasmOracle::default();

        // Set base denom price source as fixed = 1
        let price_source = WasmPriceSourceChecked::Fixed {
            price: Decimal::one(),
        };
        contract.price_sources.save(deps.storage, &msg.base_denom, &price_source)?;

        // Instantiate base oracle contract
        contract.instantiate(deps, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<WasmPriceSourceUnchecked, WasmOracleCustomExecuteMsg>,
    ) -> ContractResult<Response> {
        let contract = WasmOracle::default();
        match msg {
            ExecuteMsg::Custom(custom_msg) => match custom_msg {
                WasmOracleCustomExecuteMsg::RecordTwapSnapshots {
                    denoms,
                } => contract.execute_record_astroport_twap_snapshots(deps, env, denoms),
            },
            _ => contract.execute(deps, info, msg),
        }
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
        WasmOracle::default().query(deps, env, msg)
    }

    #[entry_point]
    pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> ContractResult<Response> {
        migrations::v2_0_0::migrate(deps)
    }
}
