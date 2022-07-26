use cosmwasm_std::{CosmosMsg, Deps, Env, StdError, StdResult, Uint128};
use osmo_bindings::{OsmosisMsg, Step, Swap, SwapAmountWithLimit};

/// Swap assets via Osmosis
pub fn construct_swap_msg(
    deps: Deps,
    env: Env,
    denom_in: &str,
    swap_amount: Uint128,
    steps: &[Step],
) -> StdResult<CosmosMsg<OsmosisMsg>> {
    // Having the same asset as offer and ask asset doesn't make any sense
    match steps.last() {
        Some(Step {
            pool_id: _,
            denom_out,
        }) => {
            if denom_in == denom_out {
                return Err(StdError::generic_err(format!(
                    "Cannot swap an asset into itself. Both assets were specified as {}",
                    denom_in
                )));
            }
        }
        None => {
            return Err(StdError::generic_err(format!(
                "Invalid swap route {:?}, the route should contain at least one step",
                steps
            )))
        }
    }

    // Swap Amount must be greater than zero
    if swap_amount.is_zero() {
        return Err(StdError::GenericErr {
            msg: "Swap amount must be strictly greater than zero".to_string(),
        });
    }

    // Get the contract balance for the offer asset
    let contract_offer_asset_balance =
        deps.querier.query_balance(env.contract.address, denom_in)?.amount;

    if swap_amount > contract_offer_asset_balance {
        return Err(StdError::generic_err(format!(
            "The amount requested for swap exceeds contract balance for the asset {}",
            denom_in
        )));
    }

    let first_swap = match steps.first() {
        Some(Step {
            pool_id,
            denom_out,
        }) => Swap::new(*pool_id, denom_in, denom_out.clone()),
        None => {
            return Err(StdError::generic_err(format!(
                "Invalid swap route {:?}, the route should contain at least one step",
                steps
            )))
        }
    };

    Ok(CosmosMsg::Custom(OsmosisMsg::Swap {
        first: first_swap,
        route: steps[1..].to_vec(),
        amount: SwapAmountWithLimit::ExactIn {
            input: swap_amount,
            min_output: Uint128::zero(),
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::Coin;
    use mars_outpost::testing::{
        assert_generic_error_message, mock_dependencies, mock_env, MockEnvParams,
    };

    #[test]
    fn test_cannot_swap_same_assets() {
        let msg = construct_swap_msg(
            mock_dependencies(&[]).as_ref(),
            mock_env(MockEnvParams::default()),
            "uosmo",
            Uint128::new(1000),
            &[Step {
                pool_id: 1,
                denom_out: "uosmo".to_string(),
            }],
        );

        assert_generic_error_message(
            msg,
            "Cannot swap an asset into itself. Both assets were specified as uosmo",
        );
    }

    #[test]
    fn test_cannot_swap_asset_with_zero_swap_amount() {
        let deps = mock_dependencies(&[Coin {
            denom: "uosmo".to_string(),
            amount: Uint128::new(100_000),
        }]);

        let msg = construct_swap_msg(
            deps.as_ref(),
            mock_env(MockEnvParams::default()),
            "uosmo",
            Uint128::zero(),
            &[Step {
                pool_id: 1,
                denom_out: "umars".to_string(),
            }],
        );
        assert_generic_error_message(msg, "Swap amount must be strictly greater than zero")
    }

    #[test]
    fn test_cannot_swap_asset_with_zero_balance() {
        let deps = mock_dependencies(&[Coin {
            denom: "uosmo".to_string(),
            amount: Uint128::zero(),
        }]);

        let msg = construct_swap_msg(
            deps.as_ref(),
            mock_env(MockEnvParams::default()),
            "uosmo",
            Uint128::new(1000),
            &[Step {
                pool_id: 1,
                denom_out: "umars".to_string(),
            }],
        );
        assert_generic_error_message(
            msg,
            "The amount requested for swap exceeds contract balance for the asset uosmo",
        )
    }

    #[test]
    fn test_cannot_swap_more_than_contract_balance() {
        let deps = mock_dependencies(&[Coin {
            denom: "somecoin".to_string(),
            amount: Uint128::new(1_000_000),
        }]);

        let msg = construct_swap_msg(
            deps.as_ref(),
            mock_env(MockEnvParams::default()),
            "somecoin",
            Uint128::new(1_000_001),
            &[Step {
                pool_id: 1,
                denom_out: "uosmo".to_string(),
            }],
        );
        assert_generic_error_message(
            msg,
            "The amount requested for swap exceeds contract balance for the asset somecoin",
        )
    }

    #[test]
    fn test_cannot_swap_with_invalid_route() {
        let deps = mock_dependencies(&[Coin {
            denom: "somecoin".to_string(),
            amount: Uint128::new(1_000_000),
        }]);

        let msg = construct_swap_msg(
            deps.as_ref(),
            mock_env(MockEnvParams::default()),
            "somecoin",
            Uint128::new(1_000_001),
            &[Step {
                pool_id: 1,
                denom_out: "uosmo".to_string(),
            }],
        );
        assert_generic_error_message(
            msg,
            "The amount requested for swap exceeds contract balance for the asset somecoin",
        )
    }
    #[test]
    fn test_swap_native_token_balance() {
        let contract_asset_balance = Uint128::new(1_000_000);
        let deps = mock_dependencies(&[Coin {
            denom: "uosmo".to_string(),
            amount: contract_asset_balance,
        }]);

        let msg = construct_swap_msg(
            deps.as_ref(),
            mock_env(MockEnvParams::default()),
            "uosmo",
            Uint128::new(500_000),
            &[Step {
                pool_id: 1,
                denom_out: "uusdc".to_string(),
            }],
        )
        .unwrap();

        assert_eq!(
            msg,
            CosmosMsg::Custom(OsmosisMsg::Swap {
                first: Swap {
                    pool_id: 1,
                    denom_in: "uosmo".to_string(),
                    denom_out: "uusdc".to_string()
                },
                route: Vec::new(),
                amount: SwapAmountWithLimit::ExactIn {
                    input: Uint128::new(500_000),
                    min_output: Uint128::zero()
                }
            })
        );
    }
}
