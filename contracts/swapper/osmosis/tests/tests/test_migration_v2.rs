use cosmwasm_std::{attr, testing::mock_env, Addr, Event};
use cw2::{ContractVersion, VersionError};
use mars_swapper_base::ContractError;
use mars_swapper_osmosis::{
    contract::{migrate, OsmosisSwap},
    migrations::v2_0_0::v1_state::{self, OwnerSetNoneProposed},
};
use mars_testing::mock_dependencies;
use mars_types::swapper::MigrateMsg;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "1.0.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V1_0_0ToV2_0_0 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-swapper-osmosis".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-swapper-osmosis", "4.1.0")
        .unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V1_0_0ToV2_0_0 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "1.0.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-swapper-osmosis", "1.0.0")
        .unwrap();

    let old_owner = "spiderman_246";
    v1_state::OWNER
        .save(
            deps.as_mut().storage,
            &v1_state::OwnerState::B(OwnerSetNoneProposed {
                owner: Addr::unchecked(old_owner),
            }),
        )
        .unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg::V1_0_0ToV2_0_0 {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "1.0.0"), attr("to_version", "2.0.1")] // to_version should be 2.0.0 but because of global current version in Cargo.toml is different
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-swapper-osmosis".to_string(),
        version: "2.0.1".to_string(), // should be 2.0.0 but global current version in Cargo.toml is different
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);

    let swapper = OsmosisSwap::default();
    let o = swapper.owner.query(deps.as_ref().storage).unwrap();
    assert_eq!(old_owner.to_string(), o.owner.unwrap());
    assert!(o.proposed.is_none());
    assert!(o.initialized);
    assert!(!o.abolished);
    assert!(o.emergency_owner.is_none());
}

#[test]
fn successful_migration_to_v2_0_2() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-swapper-osmosis", "2.0.0")
        .unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_0_0ToV2_0_1 {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.0.0"), attr("to_version", "2.0.1")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-swapper-osmosis".to_string(),
        version: "2.0.1".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
