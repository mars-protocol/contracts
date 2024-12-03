use cw2::VersionError;
use mars_incentives::{migrations::v2_0_1::migrate, ContractError};
use mars_testing::mock_dependencies;

use cosmwasm_std::{
    attr,
    testing::{mock_env, mock_info},
    Addr, Decimal, Empty,
};

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "2.0.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-incentives".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-incentives", "4.1.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "1.2.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn full_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-incentives", "1.2.0").unwrap();

    let msg = Empty {};
    let res = migrate(deps.as_mut(), mock_env(), msg).unwrap();
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "migrate"),
            attr("from_version", "1.2.0"),
            attr("to_version", "1.3.0"),
        ]
    );
}