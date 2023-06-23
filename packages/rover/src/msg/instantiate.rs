use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

use crate::adapters::{
    account_nft::AccountNftUnchecked, health::HealthContractUnchecked, oracle::OracleUnchecked,
    params::ParamsUnchecked, red_bank::RedBankUnchecked, swap::SwapperUnchecked,
    zapper::ZapperUnchecked,
};

#[cw_serde]
pub struct InstantiateMsg {
    /// The address with privileged access to update config
    pub owner: String,
    /// The Mars Protocol money market contract where we borrow assets from
    pub red_bank: RedBankUnchecked,
    /// The Mars Protocol oracle contract. We read prices of assets here.
    pub oracle: OracleUnchecked,
    /// The maximum number of unlocking positions an account can have simultaneously
    /// Note: As health checking requires looping through each, this number must not be too large.
    ///       If so, having too many could prevent the account from being liquidated due to gas constraints.
    pub max_unlocking_positions: Uint128,
    /// Helper contract for making swaps
    pub swapper: SwapperUnchecked,
    /// Helper contract for adding/removing liquidity
    pub zapper: ZapperUnchecked,
    /// Helper contract for calculating health factor
    pub health_contract: HealthContractUnchecked,
    /// Contract that stores asset and vault params
    pub params: ParamsUnchecked,
}

/// Used when you want to update fields on Instantiate config
#[cw_serde]
#[derive(Default)]
pub struct ConfigUpdates {
    pub account_nft: Option<AccountNftUnchecked>,
    pub oracle: Option<OracleUnchecked>,
    pub red_bank: Option<RedBankUnchecked>,
    pub max_unlocking_positions: Option<Uint128>,
    pub swapper: Option<SwapperUnchecked>,
    pub zapper: Option<ZapperUnchecked>,
    pub health_contract: Option<HealthContractUnchecked>,
    /// The Mars Protocol rewards-collector contract. We collect protocol fee for its account.
    pub rewards_collector: Option<String>,
}
