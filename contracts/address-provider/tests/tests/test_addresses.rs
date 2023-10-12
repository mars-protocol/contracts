use cosmwasm_std::testing::{mock_env, mock_info};
use mars_address_provider::{contract::execute, error::ContractError, state::ADDRESSES};
use mars_owner::OwnerError;
use mars_types::address_provider::{AddressResponseItem, ExecuteMsg, MarsAddressType, QueryMsg};

use super::helpers::{th_query, th_setup};

#[test]
fn setting_address_if_unauthorized() {
    let mut deps = th_setup();

    let msg = ExecuteMsg::SetAddress {
        address_type: MarsAddressType::RedBank,
        address: "osmo_red_bank".to_string(),
    };

    let err =
        execute(deps.as_mut(), mock_env(), mock_info("osmo_jake", &[]), msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Owner(OwnerError::NotOwner {}));

    // owner can set address
    execute(deps.as_mut(), mock_env(), mock_info("osmo_owner", &[]), msg).unwrap();

    let address = ADDRESSES.load(deps.as_ref().storage, MarsAddressType::RedBank.into()).unwrap();
    assert_eq!(address, "osmo_red_bank".to_string());
}

#[test]
fn setting_address_if_invalid_remote_address() {
    let mut deps = th_setup();

    let invalid_address = "mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3ze575".to_string();
    let msg = ExecuteMsg::SetAddress {
        address_type: MarsAddressType::SafetyFund,
        address: invalid_address.clone(),
    };

    let err = execute(deps.as_mut(), mock_env(), mock_info("osmo_owner", &[]), msg).unwrap_err();
    assert_eq!(err, ContractError::InvalidAddress(invalid_address));
}

#[test]
fn setting_address() {
    let mut deps = th_setup();

    let invalid_address = "mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575".to_string();
    let msg = ExecuteMsg::SetAddress {
        address_type: MarsAddressType::SafetyFund,
        address: invalid_address,
    };

    execute(deps.as_mut(), mock_env(), mock_info("osmo_owner", &[]), msg).unwrap();

    let address =
        ADDRESSES.load(deps.as_ref().storage, MarsAddressType::SafetyFund.into()).unwrap();
    assert_eq!(address, "mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575".to_string());
}

#[test]
fn querying_addresses() {
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
