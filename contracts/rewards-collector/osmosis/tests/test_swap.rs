use cosmwasm_std::{
    testing::{mock_env, MOCK_CONTRACT_ADDR},
    CosmosMsg, Decimal, Fraction, SubMsg, Uint128,
};
use mars_outpost::rewards_collector::{ConfigResponse, QueryMsg};
use mars_rewards_collector_osmosis::{contract::entry::execute, msg::ExecuteMsg};
use mars_testing::mock_info;
use osmosis_std::types::{
    cosmos::base::v1beta1::Coin,
    osmosis::{
        gamm::v1beta1::{MsgSwapExactAmountIn, SwapAmountInRoute},
        twap::v1beta1::ArithmeticTwapToNowResponse,
    },
};

mod helpers;

#[test]
fn swapping_asset_if_quering_price_fails() {
    let mut deps = helpers::setup_test();

    // Only pool_1 set, missing pool_69 and pool_420
    deps.querier.set_arithmetic_twap_price(
        1,
        "uatom",
        "uosmo",
        ArithmeticTwapToNowResponse {
            arithmetic_twap: Decimal::from_ratio(125u128, 10u128).to_string(),
        },
    );

    // Should fail because can't query price (missing price for pools) for calculating min out amount
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::SwapAsset {
            denom: "uatom".to_string(),
            amount: Some(Uint128::new(42069)),
        },
    )
    .unwrap_err();
}

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

    // amount for safety fund:   42069 * 0.25 = 10517
    // amount for fee collector: 42069 - 10517 = 31552
    // denom_in: "uatom"
    let safety_fund_route = vec![
        SwapAmountInRoute {
            pool_id: 1,
            token_out_denom: "uosmo".to_string(),
        },
        SwapAmountInRoute {
            pool_id: 69,
            token_out_denom: "uusdc".to_string(),
        },
    ];
    let safety_fund_input = Uint128::new(10517);
    // pool_1 price * pool_69 price
    let uatom_uusdc_price = uatom_uosmo_price * uosmo_uusdc_price;
    let out_amount = safety_fund_input
        .multiply_ratio(uatom_uusdc_price.numerator(), uatom_uusdc_price.denominator());
    let safety_fund_min_output = (Decimal::one() - cfg.slippage_tolerance) * out_amount;
    // denom_in: "uatom"
    let fee_collector_route = vec![
        SwapAmountInRoute {
            pool_id: 1,
            token_out_denom: "uosmo".to_string(),
        },
        SwapAmountInRoute {
            pool_id: 420,
            token_out_denom: "umars".to_string(),
        },
    ];
    let fee_collector_input = Uint128::new(31552);
    // pool_1 price * pool_420 price
    let uatom_umars_price = uatom_uosmo_price * uosmo_umars_price;
    let out_amount = fee_collector_input
        .multiply_ratio(uatom_umars_price.numerator(), uatom_umars_price.denominator());
    let fee_collector_min_output = (Decimal::one() - cfg.slippage_tolerance) * out_amount;

    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::SwapAsset {
            denom: "uatom".to_string(),
            amount: Some(Uint128::new(42069)),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 2);
    let swap_msg: CosmosMsg = MsgSwapExactAmountIn {
        sender: MOCK_CONTRACT_ADDR.to_string(),
        routes: safety_fund_route.to_vec(),
        token_in: Some(Coin {
            denom: "uatom".to_string(),
            amount: safety_fund_input.to_string(),
        }),
        token_out_min_amount: safety_fund_min_output.to_string(),
    }
    .into();
    assert_eq!(res.messages[0], SubMsg::new(swap_msg));
    let swap_msg: CosmosMsg = MsgSwapExactAmountIn {
        sender: MOCK_CONTRACT_ADDR.to_string(),
        routes: fee_collector_route.to_vec(),
        token_in: Some(Coin {
            denom: "uatom".to_string(),
            amount: fee_collector_input.to_string(),
        }),
        token_out_min_amount: fee_collector_min_output.to_string(),
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
    let swap_msg: CosmosMsg = MsgSwapExactAmountIn {
        sender: MOCK_CONTRACT_ADDR.to_string(),
        routes: vec![
            SwapAmountInRoute {
                pool_id: 69,
                token_out_denom: "uosmo".to_string(),
            },
            SwapAmountInRoute {
                pool_id: 420,
                token_out_denom: "umars".to_string(),
            },
        ],
        token_in: Some(Coin {
            denom: "uusdc".to_string(),
            amount: "926".to_string(),
        }),
        token_out_min_amount: "44".to_string(),
    }
    .into();
    assert_eq!(res.messages[0], SubMsg::new(swap_msg));
}
