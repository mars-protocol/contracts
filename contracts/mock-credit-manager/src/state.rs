use cw_storage_plus::Map;
use mars_health::HealthResponse;

pub const HEALTH_RESPONSES: Map<&str, HealthResponse> = Map::new("health_responses"); // Map<account_id, HealthResponse>
