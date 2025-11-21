use cosmwasm_std::{attr, testing::mock_env, Decimal, Empty, Event};
use cw2::{ContractVersion, VersionError};
use mars_rewards_collector_base::ContractError;
use mars_rewards_collector_osmosis::{entry::migrate, OsmosisCollector};
use mars_testing::mock_dependencies;
use mars_types::rewards_collector::{Config, RewardConfig, TransferType};

const CONTRACT: &str = "crates.io:mars-rewards-collector-osmosis";

mod previous_state {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Decimal};
    use cw_storage_plus::Item;
    use mars_types::rewards_collector::RewardConfig;

    #[cw_serde]
    pub struct Config {
        pub address_provider: Addr,
        pub safety_tax_rate: Decimal,
        pub revenue_share_tax_rate: Decimal,
        pub slippage_tolerance: Decimal,
        pub safety_fund_config: RewardConfig,
        pub revenue_share_config: RewardConfig,
        pub fee_collector_config: RewardConfig,
        pub channel_id: String,
        pub timeout_seconds: u64,
    }

    pub const CONFIG: Item<Config> = Item::new("config");
}

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

    let reward_cfg = |denom: &str| RewardConfig {
        target_denom: denom.to_string(),
        transfer_type: TransferType::Bank,
    };

    let addr_provider = deps.as_ref().api.addr_validate("addr_provider").unwrap();
    let old_config = previous_state::Config {
        address_provider: addr_provider.clone(),
        safety_tax_rate: Decimal::percent(5),
        revenue_share_tax_rate: Decimal::percent(10),
        slippage_tolerance: Decimal::percent(1),
        safety_fund_config: reward_cfg("usdc"),
        revenue_share_config: reward_cfg("usdc"),
        fee_collector_config: reward_cfg("mars"),
        channel_id: "channel-1".to_string(),
        timeout_seconds: 600,
    };

    previous_state::CONFIG.save(deps.as_mut().storage, &old_config).unwrap();

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

    let collector = OsmosisCollector::default();
    let stored_config = collector.config.load(deps.as_ref().storage).unwrap();

    assert_eq!(
        stored_config,
        Config {
            address_provider: addr_provider,
            safety_tax_rate: Decimal::percent(5),
            revenue_share_tax_rate: Decimal::percent(10),
            safety_fund_config: reward_cfg("usdc"),
            revenue_share_config: reward_cfg("usdc"),
            fee_collector_config: reward_cfg("mars"),
            channel_id: "channel-1".to_string(),
            timeout_seconds: 600,
            whitelisted_distributors: vec![],
        }
    );
}
