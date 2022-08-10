use cosmwasm_std::Event;

use mars_outpost::red_bank::Market;

pub fn build_collateral_position_changed_event(
    denom: &str,
    enabled: bool,
    user_addr: String,
) -> Event {
    Event::new("collateral_position_changed")
        .add_attribute("denom", denom)
        .add_attribute("using_as_collateral", enabled.to_string())
        .add_attribute("user", user_addr)
}

pub fn build_debt_position_changed_event(denom: &str, enabled: bool, user_addr: String) -> Event {
    Event::new("debt_position_changed")
        .add_attribute("denom", denom)
        .add_attribute("borrowing", enabled.to_string())
        .add_attribute("user", user_addr)
}

pub fn build_interests_updated_event(denom: &str, market: &Market) -> Event {
    Event::new("interests_updated")
        .add_attribute("denom", denom)
        .add_attribute("borrow_index", market.borrow_index.to_string())
        .add_attribute("liquidity_index", market.liquidity_index.to_string())
        .add_attribute("borrow_rate", market.borrow_rate.to_string())
        .add_attribute("liquidity_rate", market.liquidity_rate.to_string())
}
