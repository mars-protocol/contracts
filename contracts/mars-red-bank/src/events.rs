use cosmwasm_std::Event;

pub fn build_collateral_position_changed_event(
    label: &str,
    enabled: bool,
    user_addr: String,
) -> Event {
    Event::new("collateral_position_changed")
        .add_attribute("asset", label)
        .add_attribute("using_as_collateral", enabled.to_string())
        .add_attribute("user", user_addr)
}

pub fn build_debt_position_changed_event(label: &str, enabled: bool, user_addr: String) -> Event {
    Event::new("debt_position_changed")
        .add_attribute("asset", label)
        .add_attribute("borrowing", enabled.to_string())
        .add_attribute("user", user_addr)
}
