use std::fmt;

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum AccountKind {
    Default,
    HighLeveredStrategy,
}

impl fmt::Display for AccountKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
