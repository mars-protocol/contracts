use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Api, Coin, CosmosMsg, Decimal, Empty, StdResult, WasmMsg,
};

use crate::swapper::{ExecuteMsg, SwapperRoute};

#[cw_serde]
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
        route: Option<SwapperRoute>,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: self.address().to_string(),
            msg: to_json_binary(&ExecuteMsg::<Empty, Empty>::SwapExactIn {
                coin_in: coin_in.clone(),
                denom_out: denom_out.to_string(),
                slippage,
                route,
            })?,
            funds: vec![coin_in.clone()],
        }))
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::MockApi;

    use super::*;
    use crate::swapper::{AstroRoute, AstroSwap, OsmoRoute, OsmoSwap};

    #[test]
    fn test_swapper_unchecked_from_swapper() {
        let swapper = Swapper::new(Addr::unchecked("swapper"));
        let swapper_unchecked = SwapperUnchecked::from(swapper.clone());
        assert_eq!(swapper_unchecked.address(), "swapper");
        assert_eq!(swapper_unchecked.check(&MockApi::default()).unwrap(), swapper);
    }

    #[test]
    fn test_swapper_unchecked_check() {
        let swapper = SwapperUnchecked::new("swapper".to_string());
        assert_eq!(
            swapper.check(&MockApi::default()).unwrap(),
            Swapper::new(Addr::unchecked("swapper".to_string()))
        );
    }

    #[test]
    fn test_new_and_address() {
        // Swapper
        let swapper = Swapper::new(Addr::unchecked("swapper"));
        assert_eq!(swapper.address(), &Addr::unchecked("swapper"));

        // SwapperUnchecked
        let swapper_unchecked = SwapperUnchecked::new("swapper".to_string());
        assert_eq!(swapper_unchecked.address(), "swapper");
    }

    #[test]
    fn test_swapper_swap_exact_in_msg() {
        let swapper = Swapper::new(Addr::unchecked("swapper"));
        let coin_in = Coin::new(100, "in");
        let denom_out = "out";
        let slippage = Decimal::percent(1);

        let route = SwapperRoute::Osmo(OsmoRoute {
            swaps: vec![
                OsmoSwap {
                    pool_id: 101,
                    to: "aaa".to_string(),
                },
                OsmoSwap {
                    pool_id: 201,
                    to: "out".to_string(),
                },
            ],
        });
        let msg =
            swapper.swap_exact_in_msg(&coin_in, denom_out, slippage, Some(route.clone())).unwrap();
        assert_eq!(
            msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "swapper".to_string(),
                msg: to_json_binary(&ExecuteMsg::<Empty, Empty>::SwapExactIn {
                    coin_in: coin_in.clone(),
                    denom_out: denom_out.to_string(),
                    slippage,
                    route: Some(route)
                })
                .unwrap(),
                funds: vec![coin_in.clone()],
            })
        );

        let route = SwapperRoute::Astro(AstroRoute {
            swaps: vec![
                AstroSwap {
                    from: "aaa".to_string(),
                    to: "bbb".to_string(),
                },
                AstroSwap {
                    from: "bbb".to_string(),
                    to: "out".to_string(),
                },
            ],
        });
        let msg =
            swapper.swap_exact_in_msg(&coin_in, denom_out, slippage, Some(route.clone())).unwrap();
        assert_eq!(
            msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "swapper".to_string(),
                msg: to_json_binary(&ExecuteMsg::<Empty, Empty>::SwapExactIn {
                    coin_in: coin_in.clone(),
                    denom_out: denom_out.to_string(),
                    slippage,
                    route: Some(route)
                })
                .unwrap(),
                funds: vec![coin_in],
            })
        );
    }
}
