use cosmwasm_std::{DepsMut, Env, Response};
use cw2::{assert_contract_version, set_contract_version};
use mars_owner::OwnerInit;
use mars_rover::{error::ContractResult, msg::migrate::V2Updates};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    state::{HEALTH_CONTRACT, INCENTIVES, OWNER, PARAMS, SWAPPER},
};

const FROM_VERSION: &str = "1.0.0";

/// Taken from original Owner package version: https://github.com/mars-protocol/owner/blob/e807c6b12511987577645c8bad68cc7bd6da5398/src/owner.rs#L158
pub mod v1_state {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::Addr;
    use cw_storage_plus::Item;

    pub const ACCOUNT_NFT: Item<Addr> = Item::new("account_nft");
    pub const OWNER: Item<OwnerState> = Item::new("owner");

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
}

pub fn migrate(deps: DepsMut, env: Env, updates: V2Updates) -> ContractResult<Response> {
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

    HEALTH_CONTRACT.save(deps.storage, &updates.health_contract.check(deps.api)?)?;
    PARAMS.save(deps.storage, &updates.params.check(deps.api)?)?;
    INCENTIVES.save(deps.storage, &updates.incentives.check(deps.api, env.contract.address)?)?;
    SWAPPER.save(deps.storage, &updates.swapper.check(deps.api)?)?;

    // Owner package updated, re-initializing
    let old_owner_state = v1_state::OWNER.load(deps.storage)?;
    let old_owner = v1_state::current_owner(old_owner_state);
    v1_state::OWNER.remove(deps.storage);
    OWNER.initialize(
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
