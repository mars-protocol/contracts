use cosmwasm_std::{to_json_binary, Addr, Binary, ContractResult, QuerierResult};
use mars_types::address_provider::{AddressResponseItem, QueryMsg};

// NOTE: Addresses here are all hardcoded as we always use those to target a specific contract
// in tests. This module implicitly supposes those are used.

pub fn handle_query(contract_addr: &Addr, query: QueryMsg) -> QuerierResult {
    let address_provider = Addr::unchecked("address_provider");
    if *contract_addr != address_provider {
        panic!(
            "[mock]: Address provider request made to {contract_addr} shoud be {address_provider}"
        );
    }

    let ret: ContractResult<Binary> = match query {
        QueryMsg::Address(address_type) => {
            let res = AddressResponseItem {
                address_type,
                address: address_type.to_string(),
            };
            to_json_binary(&res).into()
        }

        QueryMsg::Addresses(address_types) => {
            let addresses = address_types
                .into_iter()
                .map(|address_type| AddressResponseItem {
                    address_type,
                    address: address_type.to_string(),
                })
                .collect::<Vec<_>>();
            to_json_binary(&addresses).into()
        }

        _ => panic!("[mock]: Unsupported address provider query"),
    };

    Ok(ret).into()
}
