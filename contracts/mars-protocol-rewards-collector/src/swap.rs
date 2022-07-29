use cosmwasm_std::{CosmosMsg, Uint128};
use osmo_bindings::{OsmosisMsg, Step, Swap, SwapAmountWithLimit};

use crate::{ContractError, ContractResult};

pub fn build_swap_msg(
    denom_in: &str,
    amount: Uint128,
    steps: &[Step],
) -> ContractResult<CosmosMsg<OsmosisMsg>> {
    let first_swap = steps
        .first()
        .map(|step| Swap::new(step.pool_id, denom_in, &step.denom_out))
        .ok_or(ContractError::InvalidSwapRoute {
            steps: steps.to_vec(),
            reason: "the route must contain at least one step".to_string(),
        })?;

    Ok(CosmosMsg::Custom(OsmosisMsg::Swap {
        first: first_swap,
        route: steps[1..].to_vec(),
        amount: SwapAmountWithLimit::ExactIn {
            input: amount,
            min_output: Uint128::zero(), // TODO: implement slippage tolerance
        },
    }))
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use cosmwasm_std::Coin;
//     use mars_outpost::testing::{
//         assert_generic_error_message, mock_dependencies, mock_env, MockEnvParams,
//     };

//     #[test]
//     fn test_cannot_swap_same_assets() {
//         let msg = build_swap_msg(
//             mock_dependencies(&[]).as_ref(),
//             mock_env(MockEnvParams::default()),
//             "uosmo",
//             Uint128::new(1000),
//             &[Step {
//                 pool_id: 1,
//                 denom_out: "uosmo".to_string(),
//             }],
//         );

//         assert_generic_error_message(
//             msg,
//             "Cannot swap an asset into itself. Both assets were specified as uosmo",
//         );
//     }

//     #[test]
//     fn test_cannot_swap_asset_with_zero_swap_amount() {
//         let deps = mock_dependencies(&[Coin {
//             denom: "uosmo".to_string(),
//             amount: Uint128::new(100_000),
//         }]);

//         let msg = build_swap_msg(
//             deps.as_ref(),
//             mock_env(MockEnvParams::default()),
//             "uosmo",
//             Uint128::zero(),
//             &[Step {
//                 pool_id: 1,
//                 denom_out: "umars".to_string(),
//             }],
//         );
//         assert_generic_error_message(msg, "Swap amount must be strictly greater than zero")
//     }

//     #[test]
//     fn test_cannot_swap_asset_with_zero_balance() {
//         let deps = mock_dependencies(&[Coin {
//             denom: "uosmo".to_string(),
//             amount: Uint128::zero(),
//         }]);

//         let msg = build_swap_msg(
//             deps.as_ref(),
//             mock_env(MockEnvParams::default()),
//             "uosmo",
//             Uint128::new(1000),
//             &[Step {
//                 pool_id: 1,
//                 denom_out: "umars".to_string(),
//             }],
//         );
//         assert_generic_error_message(
//             msg,
//             "The amount requested for swap exceeds contract balance for the asset uosmo",
//         )
//     }

//     #[test]
//     fn test_cannot_swap_more_than_contract_balance() {
//         let deps = mock_dependencies(&[Coin {
//             denom: "somecoin".to_string(),
//             amount: Uint128::new(1_000_000),
//         }]);

//         let msg = build_swap_msg(
//             deps.as_ref(),
//             mock_env(MockEnvParams::default()),
//             "somecoin",
//             Uint128::new(1_000_001),
//             &[Step {
//                 pool_id: 1,
//                 denom_out: "uosmo".to_string(),
//             }],
//         );
//         assert_generic_error_message(
//             msg,
//             "The amount requested for swap exceeds contract balance for the asset somecoin",
//         )
//     }

//     #[test]
//     fn test_cannot_swap_with_invalid_route() {
//         let deps = mock_dependencies(&[Coin {
//             denom: "somecoin".to_string(),
//             amount: Uint128::new(1_000_000),
//         }]);

//         let msg = build_swap_msg(
//             deps.as_ref(),
//             mock_env(MockEnvParams::default()),
//             "somecoin",
//             Uint128::new(1_000_001),
//             &[Step {
//                 pool_id: 1,
//                 denom_out: "uosmo".to_string(),
//             }],
//         );
//         assert_generic_error_message(
//             msg,
//             "The amount requested for swap exceeds contract balance for the asset somecoin",
//         )
//     }
//     #[test]
//     fn test_swap_native_token_balance() {
//         let contract_asset_balance = Uint128::new(1_000_000);
//         let deps = mock_dependencies(&[Coin {
//             denom: "uosmo".to_string(),
//             amount: contract_asset_balance,
//         }]);

//         let msg = build_swap_msg(
//             deps.as_ref(),
//             mock_env(MockEnvParams::default()),
//             "uosmo",
//             Uint128::new(500_000),
//             &[Step {
//                 pool_id: 1,
//                 denom_out: "uusdc".to_string(),
//             }],
//         )
//         .unwrap();

//         assert_eq!(
//             msg,
//             CosmosMsg::Custom(OsmosisMsg::Swap {
//                 first: Swap {
//                     pool_id: 1,
//                     denom_in: "uosmo".to_string(),
//                     denom_out: "uusdc".to_string()
//                 },
//                 route: Vec::new(),
//                 amount: SwapAmountWithLimit::ExactIn {
//                     input: Uint128::new(500_000),
//                     min_output: Uint128::zero()
//                 }
//             })
//         );
//     }
// }
