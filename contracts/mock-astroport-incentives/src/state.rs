use astroport_v5::incentives::IncentivesSchedule;
use cosmwasm_std::Uint128;
use cw_storage_plus::Map;

// Storage for our mock incentive schedules. Key is lp_denom, reward_asset_denom
pub const INCENTIVE_SCHEDULES: Map<(&str, &str), IncentivesSchedule> =
    Map::new("astro_incentive_schedules");
pub const ASTRO_LP_INCENTIVE_DEPOSITS: Map<(&str, &str), Uint128> =
    Map::new("astro_incentive_lp_deposits");
