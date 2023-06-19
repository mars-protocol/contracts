use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct WasmOracleCustomInitParams {
    /// The Astroport factory contract address
    pub astroport_factory: String,
}

#[cw_serde]
pub enum WasmOracleCustomExecuteMsg {
    RecordTwapSnapshots {
        denoms: Vec<String>,
    },
}

#[cw_serde]
pub struct AstroportTwapSnapshot {
    /// Timestamp of the most recent TWAP data update
    pub timestamp: u64,
    /// Cumulative price of the asset retrieved by the most recent TWAP data update
    pub price_cumulative: Uint128,
}
