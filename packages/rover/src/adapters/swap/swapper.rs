use cosmwasm_std::{to_binary, Addr, Api, Coin, CosmosMsg, Decimal, Empty, StdResult, WasmMsg};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::adapters::swap::ExecuteMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct SwapperBase<T>(T);

impl<T> SwapperBase<T> {
    pub fn new(address: T) -> SwapperBase<T> {
        SwapperBase(address)
    }

    pub fn address(&self) -> &T {
        &self.0
    }
}

pub type SwapperUnchecked = SwapperBase<String>;
pub type Swapper = SwapperBase<Addr>;

impl From<Swapper> for SwapperUnchecked {
    fn from(s: Swapper) -> Self {
        Self(s.address().to_string())
    }
}

impl SwapperUnchecked {
    pub fn check(&self, api: &dyn Api) -> StdResult<Swapper> {
        Ok(SwapperBase::new(api.addr_validate(self.address())?))
    }
}

impl Swapper {
    /// Generate message for performing a swapper
    pub fn swap_exact_in_msg(
        &self,
        coin_in: &Coin,
        denom_out: &str,
        slippage: Decimal,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().to_string(),
            msg: to_binary(&ExecuteMsg::<Empty>::SwapExactIn {
                coin_in: coin_in.clone(),
                denom_out: denom_out.to_string(),
                slippage,
            })?,
            funds: vec![coin_in.clone()],
        }))
    }
}
