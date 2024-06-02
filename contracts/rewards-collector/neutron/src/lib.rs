use std::vec;

use cosmwasm_std::{coin, Coin, CosmosMsg, Env, StdError};
use mars_rewards_collector_base::{
    contract::Collector, ContractError, ContractResult, IbcTransferMsg,
};
use neutron_sdk::{
    bindings::msg::{IbcFee, NeutronMsg},
    sudo::msg::RequestPacketTimeoutHeight,
};

pub mod migrations;

pub struct NeutronIbcMsgFactory {}

impl IbcTransferMsg<NeutronMsg> for NeutronIbcMsgFactory {
    fn ibc_transfer_msg(
        env: Env,
        to_address: String,
        amount: Coin,
        cfg: mars_types::rewards_collector::Config,
    ) -> ContractResult<CosmosMsg<NeutronMsg>> {
        let neutron_config = cfg.neutron_ibc_config.ok_or(ContractError::Std(
            StdError::generic_err("source_port must be provided for neutron"),
        ))?;
        Ok(NeutronMsg::IbcTransfer {
            source_port: neutron_config.source_port,
            source_channel: cfg.channel_id,
            token: amount,
            sender: env.contract.address.to_string(),
            receiver: to_address,
            timeout_height: RequestPacketTimeoutHeight {
                revision_number: None,
                revision_height: None,
            },
            timeout_timestamp: env.block.time.nanos() + cfg.timeout_seconds * 1_000_000_000,
            memo: "".to_string(),
            fee: IbcFee {
                recv_fee: vec![coin(0u128, "untrn")],
                ack_fee: neutron_config.acc_fee,
                timeout_fee: neutron_config.timeout_fee,
            },
        }
        .into())
    }
}

pub type NeutronCollector<'a> = Collector<'a, NeutronMsg, NeutronIbcMsgFactory>;

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::{
        entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdResult,
    };
    use cw2::set_contract_version;
    use mars_rewards_collector_base::ContractResult;
    use mars_types::rewards_collector::{ExecuteMsg, InstantiateMsg, QueryMsg};
    use neutron_sdk::bindings::msg::NeutronMsg;

    use crate::{migrations, NeutronCollector};

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
    ) -> ContractResult<Response<NeutronMsg>> {
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
