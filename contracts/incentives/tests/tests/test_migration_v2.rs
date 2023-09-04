use cosmwasm_std::{attr, testing::mock_env, Addr, Event};
use cw2::VersionError;
use mars_incentives::{
    contract::migrate,
    migrations::v2_0_0::v1_state::{self, OwnerSetNoneProposed},
    state::OWNER,
    ContractError,
};
use mars_red_bank_types::incentives::{MigrateMsg, V2Updates};
use mars_red_bank_types_old::incentives::Config;
use mars_testing::mock_dependencies;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "1.0.0").unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V1_0_0ToV2_0_0(V2Updates {
            epoch_duration: 604800,
            max_whitelisted_denoms: 10,
        }),
    )
    .unwrap_err();

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

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V1_0_0ToV2_0_0(V2Updates {
            epoch_duration: 604800,
            max_whitelisted_denoms: 10,
        }),
    )
    .unwrap_err();

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
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-incentives", "1.0.0").unwrap();

    let old_owner = "spiderman_246";
    v1_state::OWNER
        .save(
            deps.as_mut().storage,
            &v1_state::OwnerState::B(OwnerSetNoneProposed {
                owner: Addr::unchecked(old_owner),
            }),
        )
        .unwrap();

    let mars_denom = "umars";
    v1_state::CONFIG
        .save(
            deps.as_mut().storage,
            &Config {
                address_provider: Addr::unchecked("address_provider"),
                mars_denom: mars_denom.to_string(),
            },
        )
        .unwrap();

    let res = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V1_0_0ToV2_0_0(V2Updates {
            epoch_duration: 604800,
            max_whitelisted_denoms: 10,
        }),
    )
    .unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "1.0.0"), attr("to_version", "2.0.0")]
    );

    // let set_health_contract =
    //     HEALTH_CONTRACT.load(deps.as_ref().storage).unwrap().address().to_string();
    // assert_eq!(health_contract, set_health_contract);

    // let set_params = PARAMS.load(deps.as_ref().storage).unwrap().address().to_string();
    // assert_eq!(params, set_params);

    // let set_incentives = INCENTIVES.load(deps.as_ref().storage).unwrap().addr.to_string();
    // assert_eq!(incentives, set_incentives);

    // let set_swapper = SWAPPER.load(deps.as_ref().storage).unwrap().address().to_string();
    // assert_eq!(swapper, set_swapper);

    // let set_rewards = REWARDS_COLLECTOR.may_load(deps.as_ref().storage).unwrap();
    // assert_eq!(None, set_rewards);

    let o = OWNER.query(deps.as_ref().storage).unwrap();
    assert_eq!(old_owner.to_string(), o.owner.unwrap());
    assert!(o.proposed.is_none());
    assert!(o.initialized);
    assert!(!o.abolished);
    assert!(o.emergency_owner.is_none());
}
