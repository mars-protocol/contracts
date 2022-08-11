use cosmwasm_std::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Health {
    /// Total value of assets
    pub total_assets_value: Decimal,
    /// Total value of debts
    pub total_debts_value: Decimal,
    /// The sum of the value of all assets (multiplied by their liquidation threshold) over the
    /// sum of the value of all debts. Main health factor used throughout app.
    pub lqdt_health_factor: Option<Decimal>,
    /// Liquidation Health factor <= 1
    pub liquidatable: bool,
    /// The sum of the value of all assets (multiplied by their max LTV) over the sum of the value
    /// of all debts. Used to enforce a leverage limit that does not liquidate with a little volatility.
    pub max_ltv_health_factor: Option<Decimal>,
    /// Exceeding the maximum LTV that we allow users to take a new position.
    /// Uses max_ltv (instead of liquidation threshold) to calculate health factor.
    pub above_max_ltv: bool,
}
