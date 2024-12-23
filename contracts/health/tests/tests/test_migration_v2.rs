use cosmwasm_std::{attr, testing::mock_env, Empty, Event};
use cw2::{ContractVersion, VersionError};
use mars_rover_health::contract::migrate;
use mars_testing::mock_dependencies;
use mars_types::health::HealthError;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.0.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        HealthError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-rover-health".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-rover-health", "4.1.0")
        .unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        HealthError::Version(VersionError::WrongVersion {
            expected: "2.0.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-rover-health", "2.0.0")
        .unwrap();

    let res = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.0.0"), attr("to_version", "2.1.0")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-rover-health".to_string(),
        version: "2.1.0".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
