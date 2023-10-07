use cosmwasm_schema::cw_serde;
use mars_rover::adapters::oracle::OracleUnchecked;

#[cw_serde]
pub struct LpConfig {
    pub lp_token_denom: String,
    pub lp_pair_denoms: (String, String),
}

#[cw_serde]
pub struct InstantiateMsg {
    pub oracle: OracleUnchecked,
    pub lp_configs: Vec<LpConfig>,
}
