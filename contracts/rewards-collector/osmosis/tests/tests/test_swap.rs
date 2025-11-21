use cosmwasm_std::{
    coin, testing::mock_env, to_json_binary, CosmosMsg, Decimal, Empty, SubMsg, Uint128, WasmMsg,
};
use mars_rewards_collector_osmosis::entry::execute;
use mars_testing::mock_info;
use mars_types::{
    rewards_collector::{ConfigResponse, ExecuteMsg, QueryMsg},
    swapper::{self, OsmoRoute, OsmoSwap, SwapperRoute},
};

use super::helpers;

#[test]
fn swapping_asset() {
    let mut deps = helpers::setup_test();

    let cfg: ConfigResponse = helpers::query(deps.as_ref(), QueryMsg::Config {});

    let usdc_denom = "uusdc".to_string();
    let mars_denom = "umars".to_string();
    let atom_denom = "uatom".to_string();

    let uusdc_usd_price = Decimal::one();
    let umars_uusdc_price = Decimal::from_ratio(5u128, 10u128); // 0.5 uusdc = 1 umars
    let uatom_uusdc_price = Decimal::from_ratio(125u128, 10u128); // 12.5 uusd = 1 uatom

    deps.querier.set_oracle_price(&usdc_denom, uusdc_usd_price);
    deps.querier.set_oracle_price(&mars_denom, umars_uusdc_price);
    deps.querier.set_oracle_price(&atom_denom, uatom_uusdc_price);

    deps.querier.set_swapper_estimate_price(&mars_denom, umars_uusdc_price);
    deps.querier.set_swapper_estimate_price(&atom_denom, uatom_uusdc_price);
    deps.querier.set_swapper_estimate_price(&usdc_denom, uusdc_usd_price);

    let safety_fund_input = Uint128::new(14724);
    let fee_collector_input = Uint128::new(27345);

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
                    to: cfg.safety_fund_config.target_denom.to_string(),
                }],
            })),
            fee_collector_route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 69,
                    to: cfg.fee_collector_config.target_denom.to_string(),
                }],
            })),
            safety_fund_min_receive: Some(Uint128::new(178528)),
            fee_collector_min_receive: Some(Uint128::new(663140)),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);

    let swap_msg: CosmosMsg = WasmMsg::Execute {
        contract_addr: "swapper".to_string(),
        msg: to_json_binary(&swapper::ExecuteMsg::<Empty, Empty>::SwapExactIn {
            coin_in: coin(safety_fund_input.u128(), "uatom"),
            denom_out: cfg.safety_fund_config.target_denom.to_string(),
            min_receive: Uint128::new(178528),
            route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 12,
                    to: cfg.safety_fund_config.target_denom.to_string(),
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
            denom_out: cfg.fee_collector_config.target_denom.to_string(),
            min_receive: Uint128::new(663140),
            route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 69,
                    to: cfg.fee_collector_config.target_denom.to_string(),
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
/// - safety_fund_denom = USDC
/// - revenue_share_denom = USDC
///
/// In this test, we make sure the safety fund part of the swap is properly
/// skipped.
///
/// See this issue for more explanation:
/// https://github.com/mars-protocol/red-bank/issues/166
#[test]
fn skipping_swap_if_denom_matches() {
    let mut deps = helpers::setup_test();

    let usdc_denom = "uusdc".to_string();
    let mars_denom = "umars".to_string();
    let atom_denom = "uatom".to_string();

    let uusdc_usd_price = Decimal::one();
    let umars_uusdc_price = Decimal::from_ratio(5u128, 10u128); // 0.5 uusdc = 1 umars
    let uatom_uusdc_price = Decimal::from_ratio(125u128, 10u128); // 12.5 uusd = 1 uatom

    deps.querier.set_oracle_price(&usdc_denom, uusdc_usd_price);
    deps.querier.set_oracle_price(&mars_denom, umars_uusdc_price);
    deps.querier.set_oracle_price(&atom_denom, uatom_uusdc_price);

    deps.querier.set_swapper_estimate_price(&mars_denom, umars_uusdc_price);
    deps.querier.set_swapper_estimate_price(&atom_denom, uatom_uusdc_price);
    deps.querier.set_swapper_estimate_price(&usdc_denom, uusdc_usd_price);

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::SwapAsset {
            denom: usdc_denom.to_string(),
            amount: None,
            safety_fund_route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 12,
                    to: usdc_denom.to_string(),
                }],
            })),
            fee_collector_route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 69,
                    to: mars_denom.to_string(),
                }],
            })),
            safety_fund_min_receive: Some(Uint128::new(1822)),
            fee_collector_min_receive: Some(Uint128::new(4458)),
        },
    )
    .unwrap();

    // the response should only contain one swap message, from USDC to MARS, for
    // the fee collector.
    //
    // the USDC --> USDC swap for safety fund and revenue share should be skipped.
    assert_eq!(res.messages.len(), 1);

    // amount of ATOM the contract held prior to swap: 1234
    //
    // amount for safety fund:   1234 * 0.25 = 308
    // amount for revenue share: 1234 * 0.1 = 123
    // amount for fee collector: 1234 - 308 = 803
    //
    // 1 uusdc = 0.1 uosmo
    // 1 uosmo = 0.5 umars
    // slippage tolerance: 3%
    // min out amount: 926 * 0.1 * 0.5 * (1 - 0.03) = 44
    let swap_msg: CosmosMsg = WasmMsg::Execute {
        contract_addr: "swapper".to_string(),
        msg: to_json_binary(&swapper::ExecuteMsg::<Empty, Empty>::SwapExactIn {
            coin_in: coin(803u128, "uusdc"),
            denom_out: "umars".to_string(),
            min_receive: Uint128::new(4458),
            route: Some(SwapperRoute::Osmo(OsmoRoute {
                swaps: vec![OsmoSwap {
                    pool_id: 69,
                    to: mars_denom.to_string(),
                }],
            })),
        })
        .unwrap(),
        funds: vec![coin(803u128, usdc_denom)],
    }
    .into();
    assert_eq!(res.messages[0], SubMsg::new(swap_msg));
}
