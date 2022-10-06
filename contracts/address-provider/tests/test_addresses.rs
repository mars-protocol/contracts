use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::Addr;
use mars_address_provider::contract::execute;
use mars_address_provider::error::ContractError;
use mars_address_provider::state::{CONTRACTS, GOVERNANCE};

use mars_outpost::address_provider::{
    ContractAddressResponse, ExecuteMsg, GovAddressResponse, MarsContract, MarsGov, QueryMsg,
};

use crate::helpers::{th_query, th_setup};

mod helpers;

#[test]
fn test_setting_contract_address() {
    let mut deps = th_setup();

    let msg = ExecuteMsg::SetContractAddress {
        contract: MarsContract::RedBank,
        address: "red_bank".to_string(),
    };

    // non-owner cannot set address
    let err = execute(deps.as_mut(), mock_env(), mock_info("jake", &[]), msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized);

    // owner can set address
    execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

    let address = CONTRACTS.load(deps.as_ref().storage, MarsContract::RedBank.into()).unwrap();
    assert_eq!(address, "red_bank".to_string());
}

#[test]
fn test_querying_contract_addresses() {
    let mut deps = th_setup();

    CONTRACTS
        .save(
            deps.as_mut().storage,
            MarsContract::Incentives.into(),
            &Addr::unchecked("incentives"),
        )
        .unwrap();
    CONTRACTS
        .save(deps.as_mut().storage, MarsContract::Oracle.into(), &Addr::unchecked("oracle"))
        .unwrap();
    CONTRACTS
        .save(deps.as_mut().storage, MarsContract::RedBank.into(), &Addr::unchecked("red_bank"))
        .unwrap();

    let res: ContractAddressResponse =
        th_query(deps.as_ref(), QueryMsg::ContractAddress(MarsContract::Incentives));
    assert_eq!(
        res,
        ContractAddressResponse {
            contract: MarsContract::Incentives,
            address: Addr::unchecked("incentives")
        }
    );

    let res: Vec<ContractAddressResponse> = th_query(
        deps.as_ref(),
        QueryMsg::ContractAddresses(vec![MarsContract::Incentives, MarsContract::RedBank]),
    );
    assert_eq!(
        res,
        vec![
            ContractAddressResponse {
                contract: MarsContract::Incentives,
                address: Addr::unchecked("incentives")
            },
            ContractAddressResponse {
                contract: MarsContract::RedBank,
                address: Addr::unchecked("red_bank")
            }
        ]
    );

    let res: Vec<ContractAddressResponse> = th_query(
        deps.as_ref(),
        QueryMsg::AllContractAddresses {
            start_after: None,
            limit: Some(2),
        },
    );
    assert_eq!(
        res,
        vec![
            ContractAddressResponse {
                contract: MarsContract::Incentives,
                address: Addr::unchecked("incentives")
            },
            ContractAddressResponse {
                contract: MarsContract::Oracle,
                address: Addr::unchecked("oracle")
            }
        ]
    );

    let res: Vec<ContractAddressResponse> = th_query(
        deps.as_ref(),
        QueryMsg::AllContractAddresses {
            start_after: Some(MarsContract::Oracle),
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![ContractAddressResponse {
            contract: MarsContract::RedBank,
            address: Addr::unchecked("red_bank")
        }]
    );
}

#[test]
fn test_setting_governance_address() {
    let mut deps = th_setup();

    let msg = ExecuteMsg::SetGovAddress {
        gov: MarsGov::SafetyFund,
        address: "mars_safety_fund".to_string(),
    };

    // non-owner cannot set address
    let err = execute(deps.as_mut(), mock_env(), mock_info("jake", &[]), msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized);

    // owner can set address
    execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

    let address = GOVERNANCE.load(deps.as_ref().storage, MarsGov::SafetyFund.into()).unwrap();
    assert_eq!(address, "mars_safety_fund".to_string());
}

#[test]
fn test_querying_governance_addresses() {
    let mut deps = th_setup();

    GOVERNANCE
        .save(
            deps.as_mut().storage,
            MarsGov::ProtocolAdmin.into(),
            &"mars_protocol_admin".to_string(),
        )
        .unwrap();
    GOVERNANCE
        .save(deps.as_mut().storage, MarsGov::SafetyFund.into(), &"mars_safety_fund".to_string())
        .unwrap();
    GOVERNANCE
        .save(
            deps.as_mut().storage,
            MarsGov::FeeCollector.into(),
            &"mars_fee_collector".to_string(),
        )
        .unwrap();

    let res: GovAddressResponse =
        th_query(deps.as_ref(), QueryMsg::GovAddress(MarsGov::FeeCollector));
    assert_eq!(
        res,
        GovAddressResponse {
            gov: MarsGov::FeeCollector,
            address: "mars_fee_collector".to_string()
        }
    );

    let res: Vec<GovAddressResponse> = th_query(
        deps.as_ref(),
        QueryMsg::GovAddresses(vec![MarsGov::FeeCollector, MarsGov::SafetyFund]),
    );
    assert_eq!(
        res,
        vec![
            GovAddressResponse {
                gov: MarsGov::FeeCollector,
                address: "mars_fee_collector".to_string()
            },
            GovAddressResponse {
                gov: MarsGov::SafetyFund,
                address: "mars_safety_fund".to_string()
            }
        ]
    );

    let res: Vec<GovAddressResponse> = th_query(
        deps.as_ref(),
        QueryMsg::AllGovAddresses {
            start_after: None,
            limit: Some(1),
        },
    );
    assert_eq!(
        res,
        vec![GovAddressResponse {
            gov: MarsGov::FeeCollector,
            address: "mars_fee_collector".to_string()
        },]
    );

    let res: Vec<GovAddressResponse> = th_query(
        deps.as_ref(),
        QueryMsg::AllGovAddresses {
            start_after: Some(MarsGov::ProtocolAdmin),
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![GovAddressResponse {
            gov: MarsGov::SafetyFund,
            address: "mars_safety_fund".to_string()
        }]
    );
}
