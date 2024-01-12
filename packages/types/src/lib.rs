pub mod account_nft;
pub mod adapters;
pub mod address_provider;
pub mod credit_manager;
pub mod error;
pub mod health;
pub mod incentives;
pub mod keys;
pub mod oracle;
pub mod params;
pub mod red_bank;
pub mod rewards_collector;
pub mod swapper;
pub mod traits;
pub mod zapper;

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct PaginationResponse<T> {
    pub data: Vec<T>,
    pub metadata: Metadata,
}

#[cw_serde]
pub struct Metadata {
    pub has_more: bool,
}
