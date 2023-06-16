use cw_storage_plus::Map;
use mars_rover_health_types::HealthResponse;

pub const HEALTH_RESPONSES: Map<(&str, &str), HealthResponse> = Map::new("health_responses"); // Map<(account_id, AccountKind string), HealthResponse>
