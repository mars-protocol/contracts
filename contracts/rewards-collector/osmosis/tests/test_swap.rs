use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{CosmosMsg, Decimal, SubMsg, Uint128};

use mars_outpost::rewards_collector::{Config, QueryMsg};
use osmo_bindings::{OsmosisMsg, Step, Swap, SwapAmount, SwapAmountWithLimit, SwapResponse};

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
    let safety_fund_first_swap = Swap {
        pool_id: 1,
        denom_in: "uatom".to_string(),
        denom_out: "uosmo".to_string(),
    };
    let safety_fund_route = vec![Step {
        pool_id: 69,
        denom_out: "uusdc".to_string(),
    }];
    let safety_fund_input = Uint128::new(10517);
    let safety_fund_min_output = (Decimal::one() - cfg.slippage_tolerance) * safety_fund_input;
    deps.querier.set_estimate_swap(
        &safety_fund_first_swap,
        &safety_fund_route,
        SwapResponse {
            amount: SwapAmount::Out(safety_fund_input),
        },
    );
    let fee_collector_first_swap = Swap {
        pool_id: 1,
        denom_in: "uatom".to_string(),
        denom_out: "uosmo".to_string(),
    };
    let fee_collector_route = vec![Step {
        pool_id: 420,
        denom_out: "umars".to_string(),
    }];
    let fee_collector_input = Uint128::new(31552);
    let fee_collector_min_output = (Decimal::one() - cfg.slippage_tolerance) * fee_collector_input;
    deps.querier.set_estimate_swap(
        &fee_collector_first_swap,
        &fee_collector_route,
        SwapResponse {
            amount: SwapAmount::Out(fee_collector_input),
        },
    );

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
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Custom(OsmosisMsg::Swap {
            first: safety_fund_first_swap,
            route: safety_fund_route,
            amount: SwapAmountWithLimit::ExactIn {
                input: safety_fund_input,
                min_output: safety_fund_min_output
            }
        }))
    );
    assert_eq!(
        res.messages[1],
        SubMsg::new(CosmosMsg::Custom(OsmosisMsg::Swap {
            first: fee_collector_first_swap,
            route: fee_collector_route,
            amount: SwapAmountWithLimit::ExactIn {
                input: fee_collector_input,
                min_output: fee_collector_min_output
            }
        }))
    );
}
