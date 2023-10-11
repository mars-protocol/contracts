use cosmwasm_std::{
    attr,
    testing::{mock_dependencies, mock_env},
    Empty, Event,
};
use cw2::{get_contract_version, set_contract_version, ContractVersion, VersionError};
use mars_zapper_base::ContractError;
use mars_zapper_osmosis::contract::migrate;

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
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-zapper-osmosis".to_string(),
            found: "WRONG_CONTRACT_NAME".to_string()
        }),
        err
    );
}

#[test]
fn invalid_contract_version() {
    let mut deps = mock_dependencies();
    let env = mock_env();

    let old_contract_version = ContractVersion {
        contract: "crates.io:mars-zapper-osmosis".to_string(),
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
        ContractError::Version(VersionError::WrongVersion {
            expected: "1.0.0".to_string(),
            found: "4.4.5".to_string()
        }),
        err
    );
}

#[test]
fn proper_migration() {
    let mut deps = mock_dependencies();

    let old_contract_version = ContractVersion {
        contract: "crates.io:mars-zapper-osmosis".to_string(),
        version: "1.0.0".to_string(),
    };

    set_contract_version(
        deps.as_mut().storage,
        old_contract_version.contract,
        old_contract_version.version,
    )
    .unwrap();

    let res = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap();

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-zapper-osmosis".to_string(),
        version: "2.0.0".to_string(),
    };
    assert_eq!(get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "1.0.0"), attr("to_version", "2.0.0")]
    );
}
