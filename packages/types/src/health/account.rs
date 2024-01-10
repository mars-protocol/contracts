use std::fmt;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal};
#[cfg(feature = "javascript")]
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
#[cfg_attr(feature = "javascript", derive(Tsify))]
#[cfg_attr(feature = "javascript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum BorrowTarget {
    Deposit,
    Wallet,
    Vault {
        address: Addr,
    },
    Swap {
        denom_out: String,
        slippage: Decimal,
    },
}

#[cw_serde]
#[cfg_attr(feature = "javascript", derive(Tsify))]
#[cfg_attr(feature = "javascript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum SwapKind {
    Default,
    Margin,
}

#[cw_serde]
#[cfg_attr(feature = "javascript", derive(Tsify))]
#[cfg_attr(feature = "javascript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct Slippage(Decimal);

impl Slippage {
    pub fn as_decimal(&self) -> Decimal {
        self.0
    }
}

#[cw_serde]
#[cfg_attr(feature = "javascript", derive(Tsify))]
#[cfg_attr(feature = "javascript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum LiquidationPriceKind {
    Asset,
    Debt,
}
