use cosmwasm_std::{
    attr,
    testing::{mock_dependencies, mock_env, mock_info},
    Addr, Empty, Event,
};
use cw2::{get_contract_version, set_contract_version, ContractVersion};
use cw721_base::{Cw721Contract, Ownership, QueryMsg};
use cw721_base_v16::{
    msg::InstantiateMsg as Cw721v16InstantiateMsg, Cw721Contract as Cw721ContractV16,
};
use mars_account_nft::{contract::migrate, error::ContractError::MigrationError};

pub mod helpers;

#[test]
fn invalid_contract_name() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let old_contract_version = ContractVersion {
        contract: "WRONG_CONTRACT_NAME".to_string(),
        version: "1.0.0".to_string(),
    };

    set_contract_version(
        deps.as_mut().storage,
        old_contract_version.contract.clone(),
        old_contract_version.version,
    )
    .unwrap();

    let err = migrate(deps.as_mut(), env, Empty {}).unwrap_err();
    assert_eq!(
        MigrationError {
            reason: "Wrong contract. Expected: mars-account-nft, Found: WRONG_CONTRACT_NAME"
                .to_string()
        },
        err
    );
}

#[test]
fn invalid_contract_version() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let old_contract_version = ContractVersion {
        contract: "mars-account-nft".to_string(),
        version: "4.4.5".to_string(),
    };

    set_contract_version(
        deps.as_mut().storage,
        old_contract_version.contract.clone(),
        old_contract_version.version,
    )
    .unwrap();

    let err = migrate(deps.as_mut(), env, Empty {}).unwrap_err();
    assert_eq!(
        MigrationError {
            reason: "Wrong version. Expected: 1.0.0, Found: 4.4.5".to_string()
        },
        err
    );
}

#[test]
fn proper_migration() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let minter = "nft-minter-abc";

    let old_contract_version = ContractVersion {
        contract: "mars-account-nft".to_string(),
        version: "1.0.0".to_string(),
    };

    let info = mock_info("creator", &[]);
    Cw721ContractV16::<Empty, Empty, Empty, Empty>::default()
        .instantiate(
            deps.as_mut(),
            env.clone(),
            info,
            Cw721v16InstantiateMsg {
                name: "nft-contract".to_string(),
                symbol: "xyz".to_string(),
                minter: minter.to_string(),
            },
        )
        .unwrap();

    set_contract_version(
        deps.as_mut().storage,
        old_contract_version.contract.clone(),
        old_contract_version.version.clone(),
    )
    .unwrap();

    assert_eq!(get_contract_version(deps.as_ref().storage).unwrap(), old_contract_version);

    let res = migrate(deps.as_mut(), env.clone(), Empty {}).unwrap();

    let new_contract_version = ContractVersion {
        contract: "mars-account-nft".to_string(),
        version: "2.0.0".to_string(),
    };
    assert_eq!(get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate"),
            attr("from_version", "0.16.0"),
            attr("to_version", "0.17.0"),
            attr("old_minter", minter),
            attr("owner", minter),
            attr("pending_owner", "none"),
            attr("pending_expiry", "none"),
        ]
    );

    let binary = Cw721Contract::<Empty, Empty, Empty, Empty>::default()
        .query(deps.as_ref(), env, QueryMsg::Ownership {})
        .unwrap();

    let ownership = serde_json::from_slice::<Ownership<Addr>>(binary.as_slice()).unwrap();

    assert_eq!(ownership.owner, Some(Addr::unchecked(minter)));
    assert_eq!(ownership.pending_owner, None);
    assert_eq!(ownership.pending_expiry, None);
}
