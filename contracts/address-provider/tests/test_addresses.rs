use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::Addr;
use mars_address_provider::contract::execute;
use mars_address_provider::error::ContractError;
use mars_address_provider::state::{LOCAL_ADDRESSES, REMOTE_ADDRESSES};

use mars_outpost::address_provider::{
    ExecuteMsg, LocalAddressResponse, MarsLocal, MarsRemote, QueryMsg, RemoteAddressResponse,
};

use crate::helpers::{th_query, th_setup};

mod helpers;

#[test]
fn test_setting_local_address() {
    let mut deps = th_setup();

    let msg = ExecuteMsg::SetLocalAddress {
        local: MarsLocal::RedBank,
        address: "red_bank".to_string(),
    };

    // non-owner cannot set address
    let err = execute(deps.as_mut(), mock_env(), mock_info("jake", &[]), msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized);

    // owner can set address
    execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

    let address = LOCAL_ADDRESSES.load(deps.as_ref().storage, MarsLocal::RedBank.into()).unwrap();
    assert_eq!(address, "red_bank".to_string());
}

#[test]
fn test_querying_local_addresses() {
    let mut deps = th_setup();

    LOCAL_ADDRESSES
        .save(deps.as_mut().storage, MarsLocal::Incentives.into(), &Addr::unchecked("incentives"))
        .unwrap();
    LOCAL_ADDRESSES
        .save(deps.as_mut().storage, MarsLocal::Oracle.into(), &Addr::unchecked("oracle"))
        .unwrap();
    LOCAL_ADDRESSES
        .save(deps.as_mut().storage, MarsLocal::RedBank.into(), &Addr::unchecked("red_bank"))
        .unwrap();

    let res: LocalAddressResponse =
        th_query(deps.as_ref(), QueryMsg::LocalAddress(MarsLocal::Incentives));
    assert_eq!(
        res,
        LocalAddressResponse {
            local: MarsLocal::Incentives,
            address: Addr::unchecked("incentives")
        }
    );

    let res: Vec<LocalAddressResponse> = th_query(
        deps.as_ref(),
        QueryMsg::LocalAddresses(vec![MarsLocal::Incentives, MarsLocal::RedBank]),
    );
    assert_eq!(
        res,
        vec![
            LocalAddressResponse {
                local: MarsLocal::Incentives,
                address: Addr::unchecked("incentives")
            },
            LocalAddressResponse {
                local: MarsLocal::RedBank,
                address: Addr::unchecked("red_bank")
            }
        ]
    );

    let res: Vec<LocalAddressResponse> = th_query(
        deps.as_ref(),
        QueryMsg::AllLocalAddresses {
            start_after: None,
            limit: Some(2),
        },
    );
    assert_eq!(
        res,
        vec![
            LocalAddressResponse {
                local: MarsLocal::Incentives,
                address: Addr::unchecked("incentives")
            },
            LocalAddressResponse {
                local: MarsLocal::Oracle,
                address: Addr::unchecked("oracle")
            }
        ]
    );

    let res: Vec<LocalAddressResponse> = th_query(
        deps.as_ref(),
        QueryMsg::AllLocalAddresses {
            start_after: Some(MarsLocal::Oracle),
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![LocalAddressResponse {
            local: MarsLocal::RedBank,
            address: Addr::unchecked("red_bank")
        }]
    );
}

#[test]
fn test_setting_remote_address() {
    let mut deps = th_setup();

    let msg = ExecuteMsg::SetRemoteAddress {
        remote: MarsRemote::SafetyFund,
        address: "mars_safety_fund".to_string(),
    };

    // non-owner cannot set address
    let err = execute(deps.as_mut(), mock_env(), mock_info("jake", &[]), msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized);

    // owner can set address
    execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

    let address =
        REMOTE_ADDRESSES.load(deps.as_ref().storage, MarsRemote::SafetyFund.into()).unwrap();
    assert_eq!(address, "mars_safety_fund".to_string());
}

#[test]
fn test_querying_remote_addresses() {
    let mut deps = th_setup();

    REMOTE_ADDRESSES
        .save(deps.as_mut().storage, MarsRemote::SafetyFund.into(), &"mars_safety_fund".to_string())
        .unwrap();
    REMOTE_ADDRESSES
        .save(
            deps.as_mut().storage,
            MarsRemote::FeeCollector.into(),
            &"mars_fee_collector".to_string(),
        )
        .unwrap();

    let res: RemoteAddressResponse =
        th_query(deps.as_ref(), QueryMsg::RemoteAddress(MarsRemote::FeeCollector));
    assert_eq!(
        res,
        RemoteAddressResponse {
            remote: MarsRemote::FeeCollector,
            address: "mars_fee_collector".to_string()
        }
    );

    let res: Vec<RemoteAddressResponse> = th_query(
        deps.as_ref(),
        QueryMsg::RemoteAddresses(vec![MarsRemote::FeeCollector, MarsRemote::SafetyFund]),
    );
    assert_eq!(
        res,
        vec![
            RemoteAddressResponse {
                remote: MarsRemote::FeeCollector,
                address: "mars_fee_collector".to_string()
            },
            RemoteAddressResponse {
                remote: MarsRemote::SafetyFund,
                address: "mars_safety_fund".to_string()
            }
        ]
    );

    let res: Vec<RemoteAddressResponse> = th_query(
        deps.as_ref(),
        QueryMsg::AllRemoteAddresses {
            start_after: None,
            limit: Some(1),
        },
    );
    assert_eq!(
        res,
        vec![RemoteAddressResponse {
            remote: MarsRemote::FeeCollector,
            address: "mars_fee_collector".to_string()
        },]
    );

    let res: Vec<RemoteAddressResponse> = th_query(
        deps.as_ref(),
        QueryMsg::AllRemoteAddresses {
            start_after: Some(MarsRemote::FeeCollector),
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![RemoteAddressResponse {
            remote: MarsRemote::SafetyFund,
            address: "mars_safety_fund".to_string()
        }]
    );
}
