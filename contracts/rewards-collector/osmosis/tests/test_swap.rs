use cosmwasm_std::testing::mock_env;
use cosmwasm_std::{CosmosMsg, SubMsg, Uint128};

use osmo_bindings::{OsmosisMsg, Step, Swap, SwapAmountWithLimit};

use mars_rewards_collector_osmosis::contract::entry::execute;
use mars_rewards_collector_osmosis::msg::ExecuteMsg;
use mars_testing::mock_info;

mod helpers;

#[test]
fn test_swapping_asset() {
    let mut deps = helpers::setup_test();

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

    // amount for safety fund:   42069 * 0.25 = 10517
    // amount for fee collector: 42069 - 10517 = 31552
    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Custom(OsmosisMsg::Swap {
            first: Swap {
                pool_id: 1,
                denom_in: "uatom".to_string(),
                denom_out: "uosmo".to_string()
            },
            route: vec![Step {
                pool_id: 69,
                denom_out: "uusdc".to_string()
            }],
            amount: SwapAmountWithLimit::ExactIn {
                input: Uint128::new(10517),
                min_output: Uint128::zero()
            }
        }))
    );
    assert_eq!(
        res.messages[1],
        SubMsg::new(CosmosMsg::Custom(OsmosisMsg::Swap {
            first: Swap {
                pool_id: 1,
                denom_in: "uatom".to_string(),
                denom_out: "uosmo".to_string()
            },
            route: vec![Step {
                pool_id: 420,
                denom_out: "umars".to_string()
            }],
            amount: SwapAmountWithLimit::ExactIn {
                input: Uint128::new(31552),
                min_output: Uint128::zero()
            }
        }))
    );
}
