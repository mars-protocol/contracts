use cw_storage_plus::Map;
use mars_types::health::HealthValuesResponse;

pub const HEALTH_RESPONSES: Map<(&str, &str), HealthValuesResponse> = Map::new("health_responses"); // Map<(account_id, AccountKind string), HealthResponse>
