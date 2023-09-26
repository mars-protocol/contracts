use cosmwasm_std::{DepsMut, Response};
use cw2::{assert_contract_version, set_contract_version};
use mars_owner::OwnerInit;
use mars_rewards_collector_base::ContractError;

use crate::entry::{OsmosisCollector, CONTRACT_NAME, CONTRACT_VERSION};

const FROM_VERSION: &str = "1.0.0";

pub mod v1_state {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Decimal};
    use cw_storage_plus::Item;

    pub const OWNER: Item<OwnerState> = Item::new("owner");
    pub const CONFIG: Item<Config> = Item::new("config");

    #[cw_serde]
    pub enum OwnerState {
        B(OwnerSetNoneProposed),
    }

    #[cw_serde]
    pub struct OwnerSetNoneProposed {
        pub owner: Addr,
    }

    pub fn current_owner(state: OwnerState) -> Addr {
        match state {
            OwnerState::B(b) => b.owner,
        }
    }

    #[cw_serde]
    pub struct Config {
        pub address_provider: Addr,
        pub safety_tax_rate: Decimal,
        pub safety_fund_denom: String,
        pub fee_collector_denom: String,
        pub channel_id: String,
        pub timeout_seconds: u64,
        pub slippage_tolerance: Decimal,
    }
}

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    // Owner package updated, re-initializing
    let old_owner_state = v1_state::OWNER.load(deps.storage)?;
    let old_owner = v1_state::current_owner(old_owner_state);
    v1_state::OWNER.remove(deps.storage);

    let collector = OsmosisCollector::default();
    collector.owner.initialize(
        deps.storage,
        deps.api,
        OwnerInit::SetInitialOwner {
            owner: old_owner.to_string(),
        },
    )?;

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}
