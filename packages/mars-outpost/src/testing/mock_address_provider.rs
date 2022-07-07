use cosmwasm_std::{to_binary, Addr, Binary, ContractResult, QuerierResult};

use crate::address_provider::msg::QueryMsg;
use crate::address_provider::MarsContract;

// NOTE: Addresses here are all hardcoded as we always use those to target a specific contract
// in tests. This module implicitly supposes those are used.

pub fn handle_query(contract_addr: &Addr, query: QueryMsg) -> QuerierResult {
    let address_provider = Addr::unchecked("address_provider");
    if *contract_addr != address_provider {
        panic!(
            "[mock]: Address provider request made to {} shoud be {}",
            contract_addr, address_provider
        );
    }

    let ret: ContractResult<Binary> = match query {
        QueryMsg::Address { contract } => to_binary(&get_contract_address(contract)).into(),

        QueryMsg::Addresses { contracts } => {
            let addresses = contracts
                .into_iter()
                .map(get_contract_address)
                .collect::<Vec<_>>();
            to_binary(&addresses).into()
        }

        _ => panic!("[mock]: Unsupported address provider query"),
    };

    Ok(ret).into()
}

fn get_contract_address(contract: MarsContract) -> Addr {
    match contract {
        MarsContract::Council => Addr::unchecked("council"),
        MarsContract::Incentives => Addr::unchecked("incentives"),
        MarsContract::SafetyFund => Addr::unchecked("safety_fund"),
        MarsContract::MarsToken => Addr::unchecked("mars_token"),
        MarsContract::Oracle => Addr::unchecked("oracle"),
        MarsContract::ProtocolAdmin => Addr::unchecked("protocol_admin"),
        MarsContract::ProtocolRewardsCollector => Addr::unchecked("protocol_rewards_collector"),
        MarsContract::RedBank => Addr::unchecked("red_bank"),
        MarsContract::Staking => Addr::unchecked("staking"),
        MarsContract::Treasury => Addr::unchecked("treasury"),
        MarsContract::Vesting => Addr::unchecked("vesting"),
        MarsContract::XMarsToken => Addr::unchecked("xmars_token"),
    }
}
