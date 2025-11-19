use cosmwasm_std::{attr, testing::mock_env, Decimal, Empty, Event};
use cw2::{ContractVersion, VersionError};
use mars_credit_manager::{contract::migrate, error::ContractError, state::SWAP_FEE};
use mars_testing::mock_dependencies;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.1.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    match err {
        ContractError::Version(VersionError::WrongContract {
            expected,
            found,
        }) => {
            assert_eq!(expected, "crates.io:mars-credit-manager".to_string());
            assert_eq!(found, "contract_xyz".to_string());
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-credit-manager", "4.1.0")
        .unwrap();

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
fn successful_migration_from_2_1_0() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-credit-manager", "2.1.0")
        .unwrap();

    let res = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.1.0"), attr("to_version", "2.2.0")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-credit-manager".to_string(),
        version: "2.2.0".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);

    // Ensure swap fee exists post-migration (zero by default if absent)
    let swap_fee = SWAP_FEE.may_load(deps.as_ref().storage).unwrap().unwrap_or_else(Decimal::zero);
    assert!(swap_fee <= Decimal::one());
}
