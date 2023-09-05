use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Decimal;
use mars_red_bank_types::oracle::ActionKind;

#[cw_serde]
pub struct CoinPrice {
    pub pricing: ActionKind,
    pub denom: String,
    pub price: Decimal,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub prices: Vec<CoinPrice>,
}

#[cw_serde]
pub enum ExecuteMsg {
    // Meant to simulate price changes for tests. Not available in prod.
    ChangePrice(CoinPrice),

    // Used to remove a price from the store. It can be used to simulate problem with the oracle for example circuit breakers are activated
    // for Default pricing. It means that the price is not available and the contract should not allow HF check.
    // This message is not available in prod.
    RemovePrice {
        denom: String,
        pricing: ActionKind,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(mars_red_bank_types::oracle::PriceResponse)]
    Price {
        denom: String,
        kind: Option<ActionKind>,
    },
}
