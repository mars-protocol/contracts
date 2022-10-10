use cosmwasm_std::testing::{mock_env, mock_info};
use mars_address_provider::contract::execute;
use mars_address_provider::error::ContractError;
use mars_address_provider::state::ADDRESSES;

use mars_outpost::address_provider::{AddressResponseItem, ExecuteMsg, MarsAddressType, QueryMsg};

use crate::helpers::{th_query, th_setup};

mod helpers;

#[test]
fn test_setting_address() {
    let mut deps = th_setup();

    let msg = ExecuteMsg::SetAddress {
        address_type: MarsAddressType::RedBank,
        address: "red_bank".to_string(),
    };

    // non-owner cannot set address
    let err = execute(deps.as_mut(), mock_env(), mock_info("jake", &[]), msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized);

    // owner can set address
    execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

    let address = ADDRESSES.load(deps.as_ref().storage, MarsAddressType::RedBank.into()).unwrap();
    assert_eq!(address, "red_bank".to_string());
}

#[test]
fn test_querying_addresses() {
    let mut deps = th_setup();

    ADDRESSES
        .save(deps.as_mut().storage, MarsAddressType::Incentives.into(), &"incentives".to_string())
        .unwrap();
    ADDRESSES
        .save(deps.as_mut().storage, MarsAddressType::Oracle.into(), &"oracle".to_string())
        .unwrap();
    ADDRESSES
        .save(deps.as_mut().storage, MarsAddressType::RedBank.into(), &"red_bank".to_string())
        .unwrap();

    let res: AddressResponseItem =
        th_query(deps.as_ref(), QueryMsg::Address(MarsAddressType::Incentives));
    assert_eq!(
        res,
        AddressResponseItem {
            address_type: MarsAddressType::Incentives,
            address: "incentives".to_string()
        }
    );

    let res: Vec<AddressResponseItem> = th_query(
        deps.as_ref(),
        QueryMsg::Addresses(vec![MarsAddressType::Incentives, MarsAddressType::RedBank]),
    );
    assert_eq!(
        res,
        vec![
            AddressResponseItem {
                address_type: MarsAddressType::Incentives,
                address: "incentives".to_string()
            },
            AddressResponseItem {
                address_type: MarsAddressType::RedBank,
                address: "red_bank".to_string()
            }
        ]
    );

    let res: Vec<AddressResponseItem> = th_query(
        deps.as_ref(),
        QueryMsg::AllAddresses {
            start_after: None,
            limit: Some(2),
        },
    );
    assert_eq!(
        res,
        vec![
            AddressResponseItem {
                address_type: MarsAddressType::Incentives,
                address: "incentives".to_string()
            },
            AddressResponseItem {
                address_type: MarsAddressType::Oracle,
                address: "oracle".to_string()
            }
        ]
    );

    let res: Vec<AddressResponseItem> = th_query(
        deps.as_ref(),
        QueryMsg::AllAddresses {
            start_after: Some(MarsAddressType::Oracle),
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![AddressResponseItem {
            address_type: MarsAddressType::RedBank,
            address: "red_bank".to_string()
        }]
    );
}
