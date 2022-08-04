use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Health {
    pub assets_value: Decimal,
    pub ltv_adjusted_assets_value: Decimal,
    pub debts_value: Decimal,
    pub health_factor: Option<Decimal>,
    pub healthy: bool,
}
