use cosmwasm_std::{Addr, Event, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};

use mars_outpost::red_bank::{Collateral, Config, Debt, Market};

use crate::events::{build_collateral_position_changed_event, build_debt_position_changed_event};

/// The contract's configurations
pub const CONFIG: Item<Config> = Item::new("config");

/// Money market for each asset, indexed by denoms
pub const MARKETS: Map<&str, Market> = Map::new("markets");

/// Scaled collateral amounts, indexed by composite key {user_addr | denom}
pub const COLLATERALS: Map<(&Addr, &str), Collateral> = Map::new("collaterals");

/// Scaled debt amounts, indexed by composite key {user_addr | denom}
pub const DEBTS: Map<(&Addr, &str), Debt> = Map::new("debts");

/// Uncollateralized loan limits, indexed by composite key {user_addr | denom}
pub const UNCOLLATERALIZED_LOAN_LIMITS: Map<(&Addr, &str), Uint128> = Map::new("limits");

pub(crate) fn increment_collateral(
    storage: &mut dyn Storage,
    addr: &Addr,
    denom: &str,
    amount_scaled: Uint128,
    default_enable: bool,
    events: Option<&mut Vec<Event>>,
) -> StdResult<()> {
    COLLATERALS.update(storage, (addr, denom), |collateral| -> StdResult<_> {
        match collateral {
            // if a collateral position already exists, simply increase the scaled collateral amount
            Some(mut collateral) => {
                collateral.amount_scaled = collateral.amount_scaled.checked_add(amount_scaled)?;
                Ok(collateral)
            }
            // otherwise, create a new collateral position with the given default status,
            // and optionally emit a `collateral_position_changed` event
            None => {
                if let Some(events) = events {
                    if default_enable {
                        events.push(build_collateral_position_changed_event(
                            denom,
                            true,
                            addr.to_string(),
                        ));
                    }
                }
                Ok(Collateral {
                    amount_scaled,
                    enabled: default_enable,
                })
            }
        }
    })?;
    Ok(())
}

pub(crate) fn deduct_collateral(
    storage: &mut dyn Storage,
    addr: &Addr,
    denom: &str,
    amount_scaled: Uint128,
    events: Option<&mut Vec<Event>>,
) -> StdResult<()> {
    let mut collateral = COLLATERALS.load(storage, (addr, denom))?;

    // if the scaled collateral amount is reduced to zero, delete the collateral position from storage,
    // and optionally emit a `collateral_position_changed` event
    if collateral.amount_scaled == amount_scaled {
        if let Some(events) = events {
            if collateral.enabled {
                events.push(build_collateral_position_changed_event(
                    denom,
                    false,
                    addr.to_string(),
                ));
            }
        }
        COLLATERALS.remove(storage, (addr, denom));
    }
    // otherwise, simply reduce the scaled collateral amount
    else {
        collateral.amount_scaled = collateral.amount_scaled.checked_sub(amount_scaled)?;
        COLLATERALS.save(storage, (addr, denom), &collateral)?;
    }

    Ok(())
}

pub(crate) fn increment_debt(
    storage: &mut dyn Storage,
    addr: &Addr,
    denom: &str,
    amount_scaled: Uint128,
    uncollateralized: bool,
    events: Option<&mut Vec<Event>>,
) -> StdResult<()> {
    DEBTS.update(storage, (addr, denom), |debt| -> StdResult<_> {
        match debt {
            // if a debt position already exists, simply increase the scaled debt amount
            Some(mut debt) => {
                debt.amount_scaled = debt.amount_scaled.checked_add(amount_scaled)?;
                Ok(debt)
            }
            // otherwise, create a new debt position with the given uncollateralized status,
            // and optionally emit a `debt_position_changed` event
            None => {
                if let Some(events) = events {
                    events.push(build_debt_position_changed_event(denom, true, addr.to_string()));
                }
                Ok(Debt {
                    amount_scaled,
                    uncollateralized,
                })
            }
        }
    })?;
    Ok(())
}

pub(crate) fn deduct_debt(
    storage: &mut dyn Storage,
    addr: &Addr,
    denom: &str,
    amount_scaled: Uint128,
    events: Option<&mut Vec<Event>>,
) -> StdResult<()> {
    let mut debt = DEBTS.load(storage, (addr, denom))?;

    // if the scaled debt amount is reduced to zero, delete the debt position from storage,
    // and optionally emit a `debt_position_changed` event
    if debt.amount_scaled == amount_scaled {
        if let Some(events) = events {
            events.push(build_debt_position_changed_event(denom, false, addr.to_string()));
        }
        DEBTS.remove(storage, (addr, denom));
    }
    // otherwise, simply reduce the scaled debt amount
    else {
        debt.amount_scaled = debt.amount_scaled.checked_sub(amount_scaled)?;
        DEBTS.save(storage, (addr, denom), &debt)?;
    }

    Ok(())
}
