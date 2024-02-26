use cosmwasm_std::{
    coin, testing::mock_env, to_json_binary, CosmosMsg, Decimal, Empty, SubMsg, Uint128, WasmMsg,
};
use mars_rewards_collector_osmosis::entry::execute;
use mars_testing::mock_info;
use mars_types::{
    rewards_collector::{ConfigResponse, ExecuteMsg, QueryMsg},
    swapper::{self, OsmoRoute, OsmoSwap, SwapperRoute},
};
use osmosis_std::types::osmosis::twap::v1beta1::ArithmeticTwapToNowResponse;

use super::{helpers, helpers::mock_instantiate_msg};

#[test]
fn swapping_asset() {
    let mut deps = helpers::setup_test();

    let uatom_uosmo_price = Decimal::from_ratio(125u128, 10u128);
    deps.querier.set_arithmetic_twap_price(
        1,
        "uatom",
        "uosmo",
        ArithmeticTwapToNowResponse {
            arithmetic_twap: uatom_uosmo_price.to_string(),
        },
    );
    let uosmo_uusdc_price = Decimal::from_ratio(10u128, 1u128);
    deps.querier.set_arithmetic_twap_price(
        69,
        "uosmo",
        "uusdc",
        ArithmeticTwapToNowResponse {
            arithmetic_twap: uosmo_uusdc_price.to_string(),
        },
    );
    let uosmo_umars_price = Decimal::from_ratio(5u128, 10u128);
    deps.querier.set_arithmetic_twap_price(
        420,
        "uosmo",
        "umars",
        ArithmeticTwapToNowResponse {
            arithmetic_twap: uosmo_umars_price.to_string(),
        },
    );

    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});

    let safety_fund_input = Uint128::new(10517);
    let fee_collector_input = Uint128::new(31552);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::SwapAsset {
            denom: "uatom".to_string(),
            amount: Some(Uint128::new(42069)),
            safety_fund_route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 12,
                    to: cfg.safety_fund_denom.to_string(),
                }],
            })),
            fee_collector_route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 69,
                    to: cfg.fee_collector_denom.to_string(),
                }],
            })),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);

    let swap_msg: CosmosMsg = WasmMsg::Execute {
        contract_addr: "swapper".to_string(),
        msg: to_json_binary(&swapper::ExecuteMsg::<Empty, Empty>::SwapExactIn {
            coin_in: coin(safety_fund_input.u128(), "uatom"),
            denom_out: cfg.safety_fund_denom.to_string(),
            slippage: cfg.slippage_tolerance,
            route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 12,
                    to: cfg.safety_fund_denom.to_string(),
                }],
            })),
        })
        .unwrap(),
        funds: vec![coin(safety_fund_input.u128(), "uatom")],
    }
    .into();
    assert_eq!(res.messages[0], SubMsg::new(swap_msg));

    let swap_msg: CosmosMsg = WasmMsg::Execute {
        contract_addr: "swapper".to_string(),
        msg: to_json_binary(&swapper::ExecuteMsg::<Empty, Empty>::SwapExactIn {
            coin_in: coin(fee_collector_input.u128(), "uatom"),
            denom_out: cfg.fee_collector_denom.to_string(),
            slippage: cfg.slippage_tolerance,
            route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 69,
                    to: cfg.fee_collector_denom,
                }],
            })),
        })
        .unwrap(),
        funds: vec![coin(fee_collector_input.u128(), "uatom")],
    }
    .into();
    assert_eq!(res.messages[1], SubMsg::new(swap_msg));
}

/// Here we test the case where the denom is already the target denom.
///
/// For example, for the Osmosis outpost, we plan to set
///
/// - fee_collector_denom = MARS
/// - safety_fund_denom = axlUSDC
///
/// For protocol revenue collected in axlUSDC, we want half to be swapped to
/// MARS and sent to the fee collector, and the other half _not swapped_ and
/// sent to safety fund.
///
/// In this test, we make sure the safety fund part of the swap is properly
/// skipped.
///
/// See this issue for more explanation:
/// https://github.com/mars-protocol/red-bank/issues/166
#[test]
fn skipping_swap_if_denom_matches() {
    let mut deps = helpers::setup_test();

    let uusdc_uosmo_price = Decimal::from_ratio(1u128, 10u128);
    deps.querier.set_arithmetic_twap_price(
        69,
        "uusdc",
        "uosmo",
        ArithmeticTwapToNowResponse {
            arithmetic_twap: uusdc_uosmo_price.to_string(),
        },
    );
    let uosmo_umars_price = Decimal::from_ratio(5u128, 10u128);
    deps.querier.set_arithmetic_twap_price(
        420,
        "uosmo",
        "umars",
        ArithmeticTwapToNowResponse {
            arithmetic_twap: uosmo_umars_price.to_string(),
        },
    );

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::SwapAsset {
            denom: "uusdc".to_string(),
            amount: None,
            safety_fund_route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 12,
                    to: "uusdc".to_string(),
                }],
            })),
            fee_collector_route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 69,
                    to: "umars".to_string(),
                }],
            })),
        },
    )
    .unwrap();

    // the response should only contain one swap message, from USDC to MARS, for
    // the fee collector.
    //
    // the USDC --> USDC swap for safety fund should be skipped.
    assert_eq!(res.messages.len(), 1);

    // amount of USDC the contract held prior to swap: 1234
    //
    // amount for safety fund:   1234 * 0.25 = 308
    // amount for fee collector: 1234 - 308 = 926
    //
    // 1 uusdc = 0.1 uosmo
    // 1 uosmo = 0.5 umars
    // slippage tolerance: 3%
    // min out amount: 926 * 0.1 * 0.5 * (1 - 0.03) = 44
    let swap_msg: CosmosMsg = WasmMsg::Execute {
        contract_addr: "swapper".to_string(),
        msg: to_json_binary(&swapper::ExecuteMsg::<Empty, Empty>::SwapExactIn {
            coin_in: coin(926u128, "uusdc"),
            denom_out: "umars".to_string(),
            slippage: mock_instantiate_msg().slippage_tolerance,
            route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 69,
                    to: "umars".to_string(),
                }],
            })),
        })
        .unwrap(),
        funds: vec![coin(926u128, "uusdc")],
    }
    .into();
    assert_eq!(res.messages[0], SubMsg::new(swap_msg));
}
