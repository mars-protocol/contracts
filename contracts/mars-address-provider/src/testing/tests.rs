use cosmwasm_std::testing::{mock_env, mock_info};

use mars_outpost::address_provider::{
    AddressResponseItem, Config, ExecuteMsg, MarsContract, QueryMsg,
};

use crate::contract::execute;
use crate::error::ContractError;
use crate::state::{CONFIG, CONTRACTS};

use super::helpers::{th_query, th_setup};

#[test]
fn proper_initialization() {
    let deps = th_setup();

    let config: Config = th_query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(config.owner, "owner".to_string());
}

#[test]
fn setting_address() {
    let mut deps = th_setup();

    let msg = ExecuteMsg::SetAddress {
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
fn transferring_ownership() {
    let mut deps = th_setup();

    let msg = ExecuteMsg::TransferOwnership {
        new_owner: "larry".to_string(),
    };

    // non-owner cannot transfer ownership
    let err = execute(deps.as_mut(), mock_env(), mock_info("jake", &[]), msg.clone()).unwrap_err();
    assert_eq!(err, ContractError::Unauthorized);

    // owner can transfer ownership
    execute(deps.as_mut(), mock_env(), mock_info("owner", &[]), msg).unwrap();

    let config = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(config.owner, "larry".to_string());
}

#[test]
fn querying_addresses() {
    let mut deps = th_setup();

    CONTRACTS
        .save(deps.as_mut().storage, MarsContract::Incentives.into(), &"incentives".to_string())
        .unwrap();
    CONTRACTS
        .save(deps.as_mut().storage, MarsContract::Oracle.into(), &"oracle".to_string())
        .unwrap();
    CONTRACTS
        .save(deps.as_mut().storage, MarsContract::RedBank.into(), &"red_bank".to_string())
        .unwrap();

    let res: AddressResponseItem =
        th_query(deps.as_ref(), QueryMsg::Address(MarsContract::Incentives));
    assert_eq!(
        res,
        AddressResponseItem {
            contract: MarsContract::Incentives,
            address: "incentives".to_string()
        }
    );

    let res: Vec<AddressResponseItem> = th_query(
        deps.as_ref(),
        QueryMsg::Addresses(vec![MarsContract::Incentives, MarsContract::RedBank]),
    );
    assert_eq!(
        res,
        vec![
            AddressResponseItem {
                contract: MarsContract::Incentives,
                address: "incentives".to_string()
            },
            AddressResponseItem {
                contract: MarsContract::RedBank,
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
                contract: MarsContract::Incentives,
                address: "incentives".to_string()
            },
            AddressResponseItem {
                contract: MarsContract::Oracle,
                address: "oracle".to_string()
            }
        ]
    );

    let res: Vec<AddressResponseItem> = th_query(
        deps.as_ref(),
        QueryMsg::AllAddresses {
            start_after: Some(MarsContract::Oracle),
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![AddressResponseItem {
            contract: MarsContract::RedBank,
            address: "red_bank".to_string()
        }]
    );
}
