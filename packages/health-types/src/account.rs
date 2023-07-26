use std::fmt;

use cosmwasm_schema::cw_serde;
use tsify::Tsify;

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

#[cw_serde]
#[derive(Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum BorrowTarget {
    Deposit,
    Wallet,
}
