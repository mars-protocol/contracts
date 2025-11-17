use cosmwasm_std::{DepsMut, Response, Storage};
use cw2::{assert_contract_version, get_contract_version, set_contract_version, VersionError};
use mars_rewards_collector_base::ContractError;
use mars_types::rewards_collector::Config;

use crate::{
    entry::{CONTRACT_NAME, CONTRACT_VERSION},
    OsmosisCollector,
};

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

const FROM_VERSION: &str = "2.1.1";

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    let contract = format!("crates.io:{CONTRACT_NAME}");
    let version = get_contract_version(deps.storage)?;

    if version.contract != contract {
        return Err(ContractError::Version(VersionError::WrongContract {
            expected: contract,
            found: version.contract,
        }));
    }

    if version.version != FROM_VERSION {
        return Err(ContractError::Version(VersionError::WrongVersion {
            expected: FROM_VERSION.to_string(),
            found: version.version,
        }));
    }

    assert_contract_version(deps.storage, &contract, FROM_VERSION)?;

    let storage: &mut dyn Storage = deps.storage;
    let collector = OsmosisCollector::default();

    let old_config = previous_state::CONFIG.load(storage)?;

    let new_config = Config {
        address_provider: old_config.address_provider,
        safety_tax_rate: old_config.safety_tax_rate,
        revenue_share_tax_rate: old_config.revenue_share_tax_rate,
        safety_fund_config: old_config.safety_fund_config,
        revenue_share_config: old_config.revenue_share_config,
        fee_collector_config: old_config.fee_collector_config,
        channel_id: old_config.channel_id,
        timeout_seconds: old_config.timeout_seconds,
        whitelisted_distributors: vec![],
    };

    new_config.validate()?;
    collector.config.save(storage, &new_config)?;

    set_contract_version(deps.storage, contract, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}
