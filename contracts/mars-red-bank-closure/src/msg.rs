use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use mars_core::asset::Asset;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Refund the first 10 users of a specific asset; call this function repeatedly to refund all users
    Refund { asset: Asset },
}
