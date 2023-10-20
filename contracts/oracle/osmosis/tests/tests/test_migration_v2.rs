use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{attr, testing::mock_env, Addr, Decimal, Event, Order, StdResult};
use cw2::{ContractVersion, VersionError};
use mars_oracle_base::ContractError;
use mars_oracle_osmosis::{
    contract::{entry::migrate, OsmosisOracle},
    migrations::v2_0_0::v1_state,
    DowntimeDetector, OsmosisPriceSourceChecked,
};
use mars_testing::mock_dependencies;
use mars_types::oracle::{MigrateMsg, V2Updates};
use osmosis_std::types::osmosis::downtimedetector::v1beta1::Downtime;
use pyth_sdk_cw::PriceIdentifier;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "1.1.0").unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V1_1_0ToV2_0_0(V2Updates {
            max_confidence: Decimal::percent(5),
            max_deviation: Decimal::percent(5),
        }),
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-oracle-osmosis".to_string(),
            found: "contract_xyz".to_string()
        })
    );

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_0_0ToV2_0_1 {}).unwrap_err();
    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-oracle-osmosis".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-oracle-osmosis", "4.1.0")
        .unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V1_1_0ToV2_0_0(V2Updates {
            max_confidence: Decimal::percent(5),
            max_deviation: Decimal::percent(5),
        }),
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "1.1.0".to_string(),
            found: "4.1.0".to_string()
        })
    );

    let err = migrate(deps.as_mut(), mock_env(), MigrateMsg::V2_0_0ToV2_0_1 {}).unwrap_err();
    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "2.0.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn successful_migration_to_v2_0_0() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-oracle-osmosis", "1.1.0")
        .unwrap();

    let pyth_contract =
        Addr::unchecked("osmo13ge29x4e2s63a8ytz2px8gurtyznmue4a69n5275692v3qn3ks8q7cwck7");

    let usd_denom = "usd";
    let usd_fixed = v1_state::OsmosisPriceSourceChecked::Fixed {
        price: Decimal::from_str("1000000").unwrap(),
    };
    v1_state::PRICE_SOURCES.save(deps.as_mut().storage, usd_denom, &usd_fixed).unwrap();

    let atom_denom = "ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2";
    let atom_price_feed_id = PriceIdentifier::from_hex(
        "b00b60f88b03a6a625a8d1c048c3f66653edf217439983d037e7222c4e612819",
    )
    .unwrap();
    let atom_pyth = v1_state::OsmosisPriceSourceChecked::Pyth {
        contract_addr: pyth_contract.clone(),
        price_feed_id: atom_price_feed_id,
        max_staleness: 60,
        denom_decimals: 6,
    };
    v1_state::PRICE_SOURCES.save(deps.as_mut().storage, atom_denom, &atom_pyth).unwrap();

    let eth_denom = "ibc/EA1D43981D5C9A1C4AAEA9C23BB1D4FA126BA9BC7020A25E0AE4AA841EA25DC5";
    let eth_price_feed_id = PriceIdentifier::from_hex(
        "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace",
    )
    .unwrap();
    let eth_pyth = v1_state::OsmosisPriceSourceChecked::Pyth {
        contract_addr: pyth_contract.clone(),
        price_feed_id: eth_price_feed_id,
        max_staleness: 80,
        denom_decimals: 18,
    };
    v1_state::PRICE_SOURCES.save(deps.as_mut().storage, eth_denom, &eth_pyth).unwrap();

    let lp_denom = "gamm/pool/704";
    let lp = v1_state::OsmosisPriceSourceChecked::XykLiquidityToken {
        pool_id: 704,
    };

    v1_state::PRICE_SOURCES.save(deps.as_mut().storage, lp_denom, &lp).unwrap();
    let statom_denom = "ibc/C140AFD542AE77BD7DCC83F13FDD8C5E5BB8C4929785E6EC2F4C636F98F17901";
    let statom_staked = v1_state::OsmosisPriceSourceChecked::StakedGeometricTwap {
        transitive_denom: atom_denom.to_string(),
        pool_id: 803,
        window_size: 1800,
        downtime_detector: Some(v1_state::DowntimeDetector {
            downtime: v1_state::Downtime::Duration30m,
            recovery: 7200,
        }),
    };
    v1_state::PRICE_SOURCES.save(deps.as_mut().storage, statom_denom, &statom_staked).unwrap();

    let max_confidence = Decimal::percent(10);
    let max_deviation = Decimal::percent(15);
    let res = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V1_1_0ToV2_0_0(V2Updates {
            max_confidence,
            max_deviation,
        }),
    )
    .unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "1.1.0"), attr("to_version", "2.0.1")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-oracle-osmosis".to_string(),
        version: "2.0.1".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);

    let oracle = OsmosisOracle::default();
    let price_sources = oracle
        .price_sources
        .range(&deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<HashMap<_, _>>>()
        .unwrap();
    assert_eq!(price_sources.len(), 5);
    assert_eq!(
        price_sources.get(usd_denom).unwrap(),
        &OsmosisPriceSourceChecked::Fixed {
            price: Decimal::from_str("1000000").unwrap()
        }
    );
    assert_eq!(
        price_sources.get(atom_denom).unwrap(),
        &OsmosisPriceSourceChecked::Pyth {
            contract_addr: pyth_contract.clone(),
            price_feed_id: atom_price_feed_id,
            max_staleness: 60,
            max_confidence,
            max_deviation,
            denom_decimals: 6
        }
    );
    assert_eq!(
        price_sources.get(eth_denom).unwrap(),
        &OsmosisPriceSourceChecked::Pyth {
            contract_addr: pyth_contract,
            price_feed_id: eth_price_feed_id,
            max_staleness: 80,
            max_confidence,
            max_deviation,
            denom_decimals: 18
        }
    );
    assert_eq!(
        price_sources.get(lp_denom).unwrap(),
        &OsmosisPriceSourceChecked::XykLiquidityToken {
            pool_id: 704
        }
    );
    assert_eq!(
        price_sources.get(statom_denom).unwrap(),
        &OsmosisPriceSourceChecked::StakedGeometricTwap {
            transitive_denom: atom_denom.to_string(),
            pool_id: 803,
            window_size: 1800,
            downtime_detector: Some(DowntimeDetector {
                downtime: Downtime::Duration30m,
                recovery: 7200
            })
        }
    );
}

#[test]
fn successful_migration_to_v2_0_1() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-oracle-osmosis", "2.0.0")
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
        contract: "crates.io:mars-oracle-osmosis".to_string(),
        version: "2.0.1".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);
}
