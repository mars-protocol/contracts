use cosmwasm_std::{attr, testing::mock_env, Addr, Decimal, Event};
use cw2::{ContractVersion, VersionError};
use mars_rewards_collector_base::ContractError;
use mars_rewards_collector_osmosis::{
    entry::{migrate, OsmosisCollector},
    migrations::v2_0_0::v1_state::{self, OwnerSetNoneProposed},
};
use mars_testing::mock_dependencies;
use mars_types::rewards_collector::MigrateMsg;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "1.0.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V1_0_0ToV2_0_0 {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-rewards-collector-osmosis".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(
        deps.as_mut().storage,
        "crates.io:mars-rewards-collector-osmosis",
        "4.1.0",
    )
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
    cw2::set_contract_version(
        deps.as_mut().storage,
        "crates.io:mars-rewards-collector-osmosis",
        "1.0.0",
    )
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

    let v1_config = v1_state::Config {
        address_provider: Addr::unchecked("address_provider"),
        safety_tax_rate: Decimal::percent(50),
        safety_fund_denom: "ibc/6F34E1BD664C36CE49ACC28E60D62559A5F96C4F9A6CCE4FC5A67B2852E24CFE"
            .to_string(),
        fee_collector_denom: "ibc/2E7368A14AC9AB7870F32CFEA687551C5064FA861868EDF7437BC877358A81F9"
            .to_string(),
        channel_id: "channel-2083".to_string(),
        timeout_seconds: 600,
        slippage_tolerance: Decimal::percent(1),
    };
    v1_state::CONFIG.save(deps.as_mut().storage, &v1_config).unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg::V1_0_0ToV2_0_0 {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "1.0.0"), attr("to_version", "2.1.0")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-rewards-collector-osmosis".to_string(),
        version: "2.1.0".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);

    let collector = OsmosisCollector::default();
    let o = collector.owner.query(deps.as_ref().storage).unwrap();
    assert_eq!(old_owner.to_string(), o.owner.unwrap());
    assert!(o.proposed.is_none());
    assert!(o.initialized);
    assert!(!o.abolished);
    assert!(o.emergency_owner.is_none());

    let config = collector.config.load(&deps.storage).unwrap();
    assert_eq!(v1_config.address_provider, config.address_provider);
    assert_eq!(v1_config.safety_tax_rate, config.safety_tax_rate);
    assert_eq!(v1_config.safety_fund_denom, config.safety_fund_denom);
    assert_eq!(v1_config.fee_collector_denom, config.fee_collector_denom);
    assert_eq!(v1_config.channel_id, config.channel_id);
    assert_eq!(v1_config.timeout_seconds, config.timeout_seconds);
    assert_eq!(v1_config.slippage_tolerance, config.slippage_tolerance);
    assert!(config.neutron_ibc_config.is_none());
}

#[test]
fn successful_migration_to_v2_1_0() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(
        deps.as_mut().storage,
        "crates.io:mars-rewards-collector-osmosis",
        "2.0.0",
    )
    .unwrap();

    let res = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_0_0ToV2_0_1 {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "2.0.0"), attr("to_version", "2.1.0")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-rewards-collector-osmosis".to_string(),
        version: "2.1.0".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
