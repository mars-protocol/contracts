use cosmwasm_std::{attr, testing::mock_env, Empty, Event};
use cw2::{ContractVersion, VersionError};
use mars_rewards_collector_base::ContractError;
use mars_rewards_collector_osmosis::entry::migrate;
use mars_testing::mock_dependencies;

const CONTRACT: &str = "crates.io:mars-rewards-collector-osmosis";

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.1.1").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    match err {
        ContractError::Version(VersionError::WrongContract {
            expected,
            found,
        }) => {
            assert_eq!(expected, CONTRACT.to_string());
            assert_eq!(found, "contract_xyz".to_string());
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT, "4.1.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    match err {
        ContractError::Version(VersionError::WrongVersion {
            found,
            ..
        }) => {
            assert_eq!(found, "4.1.0".to_string());
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn successful_migration_from_2_1_1() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, CONTRACT, "2.1.1").unwrap();

    let res = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.1.1"), attr("to_version", "2.2.0")]
    );

    let new_contract_version = ContractVersion {
        contract: CONTRACT.to_string(),
        version: "2.2.0".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
