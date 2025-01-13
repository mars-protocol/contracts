use cosmwasm_std::{attr, testing::mock_env, Addr, Empty, Event};
use cw2::{ContractVersion, VersionError};
use mars_incentives::{
    contract::migrate, migrations::v2_1_0::v1_state, state::CONFIG, ContractError,
};
use mars_testing::mock_dependencies;
use mars_types::incentives::Config;

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
            expected: "2.0.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-incentives", "2.0.0").unwrap();

    v1_state::CONFIG
        .save(
            deps.as_mut().storage,
            &v1_state::Config {
                address_provider: Addr::unchecked("addr_provider".to_string()),
                max_whitelisted_denoms: 15,
                mars_denom: "mars".to_string(),
            },
        )
        .unwrap();

    let res = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap();

    let config = CONFIG.load(deps.as_ref().storage).unwrap();
    assert_eq!(
        config,
        Config {
            address_provider: Addr::unchecked("addr_provider".to_string()),
            max_whitelisted_denoms: 15,
        }
    );

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.0.0"), attr("to_version", "2.1.0")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-incentives".to_string(),
        version: "2.1.0".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
