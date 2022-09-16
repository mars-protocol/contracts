use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{CosmosMsg, Decimal, SubMsg, Uint128};

use mars_outpost::rewards_collector::{Config, QueryMsg};
use osmosis_std::types::cosmos::base::v1beta1::Coin;
use osmosis_std::types::osmosis::gamm::v1beta1::{MsgSwapExactAmountIn, SwapAmountInRoute};

use mars_rewards_collector_osmosis::contract::entry::execute;
use mars_rewards_collector_osmosis::msg::ExecuteMsg;
use mars_testing::mock_info;

mod helpers;

#[test]
fn test_swapping_asset() {
    let mut deps = helpers::setup_test();

    let cfg: Config<String> = helpers::query(deps.as_ref(), QueryMsg::Config {});

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
    let safety_fund_min_output = (Decimal::one() - cfg.slippage_tolerance) * safety_fund_input;
    deps.querier.set_estimate_swap(safety_fund_input, &safety_fund_route);
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
    let fee_collector_min_output = (Decimal::one() - cfg.slippage_tolerance) * fee_collector_input;
    deps.querier.set_estimate_swap(fee_collector_input, &fee_collector_route);

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
    assert_eq!(res.messages.len(), 2);
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
