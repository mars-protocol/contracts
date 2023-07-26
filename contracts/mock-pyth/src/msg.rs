use cosmwasm_schema::{cw_serde, QueryResponses};
use pyth_sdk_cw::{Price, PriceFeedResponse, PriceIdentifier};

#[cw_serde]
pub struct PriceFeed {
    /// Unique identifier for this price.
    pub id: PriceIdentifier,
    /// Price.
    price: Price,
    /// Exponentially-weighted moving average (EMA) price.
    ema_price: Price,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PriceFeedResponse)]
    PriceFeed {
        id: PriceIdentifier,
    },
}
