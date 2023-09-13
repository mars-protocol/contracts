use cosmwasm_schema::cw_serde;
use cosmwasm_std::Decimal;

use crate::adapters::{
    health::HealthContractUnchecked, incentives::IncentivesUnchecked, params::ParamsUnchecked,
    swap::SwapperUnchecked,
};

#[cw_serde]
pub struct V2Updates {
    pub health_contract: HealthContractUnchecked,
    pub params: ParamsUnchecked,
    pub incentives: IncentivesUnchecked,
    pub swapper: SwapperUnchecked,
    pub max_slippage: Decimal,
}

#[cw_serde]
pub enum MigrateMsg {
    V1_0_0ToV2_0_0(V2Updates),
}
