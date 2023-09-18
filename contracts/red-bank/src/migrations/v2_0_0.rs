use cosmwasm_std::{DepsMut, Order, Response, StdResult};
use cw2::{assert_contract_version, set_contract_version};
use mars_owner::OwnerInit;
use mars_red_bank_types::{
    keys::{UserId, UserIdKey},
    red_bank::{Config, Market},
};

use crate::{
    contract::{CONTRACT_NAME, CONTRACT_VERSION},
    error::ContractError,
    state::{COLLATERALS, CONFIG, MARKETS, OWNER},
};

const FROM_VERSION: &str = "1.0.0";

pub mod v1_state {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Decimal, Uint128};
    use cw_storage_plus::{Item, Map};
    use mars_red_bank_types::red_bank::{Collateral, InterestRateModel};

    pub const OWNER: Item<OwnerState> = Item::new("owner");
    pub const CONFIG: Item<Config> = Item::new("config");
    pub const MARKETS: Map<&str, Market> = Map::new("markets");
    pub const COLLATERALS: Map<(&Addr, &str), Collateral> = Map::new("collaterals");

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
        pub close_factor: Decimal,
    }

    #[cw_serde]
    pub struct Market {
        pub denom: String,
        pub max_loan_to_value: Decimal,
        pub liquidation_threshold: Decimal,
        pub liquidation_bonus: Decimal,
        pub reserve_factor: Decimal,
        pub interest_rate_model: InterestRateModel,
        pub borrow_index: Decimal,
        pub liquidity_index: Decimal,
        pub borrow_rate: Decimal,
        pub liquidity_rate: Decimal,
        pub indexes_last_updated: u64,
        pub collateral_total_scaled: Uint128,
        pub debt_total_scaled: Uint128,
        pub deposit_enabled: bool,
        pub borrow_enabled: bool,
        pub deposit_cap: Uint128,
    }
}

pub fn migrate(deps: DepsMut) -> Result<Response, ContractError> {
    // make sure we're migrating the correct contract and from the correct version
    assert_contract_version(deps.storage, &format!("crates.io:{CONTRACT_NAME}"), FROM_VERSION)?;

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

    // Config package updated, re-initializing
    let old_config = v1_state::CONFIG.load(deps.storage)?;
    v1_state::CONFIG.remove(deps.storage);
    CONFIG.save(
        deps.storage,
        &Config {
            address_provider: old_config.address_provider,
        },
    )?;

    // Migrate markets.
    // Remove LP tokens because they are not supported in red-bank. Params for LP tokens exist in `params`contract.
    let markets = v1_state::MARKETS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    v1_state::MARKETS.clear(deps.storage);
    for (denom, market) in markets.into_iter() {
        if denom.starts_with("gamm/pool") {
            continue;
        }
        MARKETS.save(deps.storage, &denom, &market.into())?;
    }

    // Migrate collaterals, user id has address and account id in v2
    let collaterals = v1_state::COLLATERALS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    v1_state::COLLATERALS.clear(deps.storage);
    for ((user_addr, denom), collateral) in collaterals.into_iter() {
        let user_id = UserId::credit_manager(user_addr, "".to_string());
        let user_id_key: UserIdKey = user_id.try_into()?;
        COLLATERALS.save(deps.storage, (&user_id_key, &denom), &collateral)?;
    }

    set_contract_version(deps.storage, format!("crates.io:{CONTRACT_NAME}"), CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate")
        .add_attribute("from_version", FROM_VERSION)
        .add_attribute("to_version", CONTRACT_VERSION))
}

/// Few of the fields in old `Market` struct are moved to `params` contract
impl From<v1_state::Market> for Market {
    fn from(value: v1_state::Market) -> Self {
        Self {
            denom: value.denom,
            interest_rate_model: value.interest_rate_model,
            borrow_index: value.borrow_index,
            liquidity_index: value.liquidity_index,
            borrow_rate: value.borrow_rate,
            liquidity_rate: value.liquidity_rate,
            indexes_last_updated: value.indexes_last_updated,
            collateral_total_scaled: value.collateral_total_scaled,
            debt_total_scaled: value.debt_total_scaled,
            reserve_factor: value.reserve_factor,
        }
    }
}
