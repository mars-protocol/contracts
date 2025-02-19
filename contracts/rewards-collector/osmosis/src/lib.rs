use cosmwasm_std::{Coin, CosmosMsg, Empty, Env, IbcMsg, IbcTimeout};
use mars_rewards_collector_base::{contract::Collector, ContractResult, TransferMsg};
use mars_types::rewards_collector::{Config, TransferType};

pub mod migrations;

pub struct OsmosisMsgFactory {}

impl TransferMsg<Empty> for OsmosisMsgFactory {
    fn transfer_msg(
        env: &Env,
        to_address: &str,
        amount: Coin,
        cfg: &Config,
        transfer_type: &TransferType,
    ) -> ContractResult<CosmosMsg<Empty>> {
        match transfer_type {
            TransferType::Bank => Ok(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
                to_address: to_address.to_string(),
                amount: vec![amount],
            })),
            TransferType::Ibc => Ok(CosmosMsg::Ibc(IbcMsg::Transfer {
                channel_id: cfg.channel_id.to_string(),
                to_address: to_address.to_string(),
                amount,
                timeout: IbcTimeout::with_timestamp(
                    env.block.time.plus_seconds(cfg.timeout_seconds),
                ),
            })),
        }
    }
}

pub type OsmosisCollector<'a> = Collector<'a, Empty, OsmosisMsgFactory>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::{
        entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
    };
    use cw2::set_contract_version;
    use mars_rewards_collector_base::{ContractError, ContractResult};
    use mars_types::rewards_collector::{ExecuteMsg, InstantiateMsg, QueryMsg};

    use crate::{migrations, OsmosisCollector};

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

    #[entry_point]
    pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
        migrations::v2_1_0::migrate(deps)
    }
}
