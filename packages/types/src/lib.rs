pub mod address_provider;
pub mod error;
pub mod incentives;
pub mod oracle;
pub mod red_bank;
pub mod rewards_collector;
pub mod swapper;

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct PaginationResponse<T> {
    pub data: Vec<T>,
    pub metadata: Metadata,
}

#[cw_serde]
pub struct Metadata {
    pub has_more: bool,
    pub next_start_after: Option<String>,
}
