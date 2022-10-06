use cosmwasm_std::{to_binary, Addr, Binary, ContractResult, QuerierResult};

use mars_outpost::address_provider::{ContractAddressResponse, GovAddressResponse, QueryMsg};

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
        QueryMsg::ContractAddress(contract) => {
            let res = ContractAddressResponse {
                contract,
                address: Addr::unchecked(contract.to_string()),
            };
            to_binary(&res).into()
        }

        QueryMsg::ContractAddresses(contracts) => {
            let addresses = contracts
                .into_iter()
                .map(|contract| ContractAddressResponse {
                    contract,
                    address: Addr::unchecked(contract.to_string()),
                })
                .collect::<Vec<_>>();
            to_binary(&addresses).into()
        }

        QueryMsg::GovAddress(gov) => {
            let res = GovAddressResponse {
                gov,
                address: gov.to_string(),
            };
            to_binary(&res).into()
        }

        _ => panic!("[mock]: Unsupported address provider query"),
    };

    Ok(ret).into()
}
