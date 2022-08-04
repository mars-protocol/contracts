use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, IbcMsg, IbcTimeout, IbcTimeoutBlock,
    SubMsg, Timestamp, Uint128, WasmMsg,
};

use osmo_bindings::{OsmosisMsg, Step, Swap, SwapAmountWithLimit};

use mars_outpost::error::MarsError;
use mars_outpost::rewards_collector::{Config, CreateOrUpdateConfig, QueryMsg, RouteResponse};
use mars_rewards_collector_base::helpers::{stringify_option_amount, unwrap_option_amount};
use mars_rewards_collector_base::{ContractError, Route};
use mars_testing::{mock_env as mock_env_at_height_and_time, mock_info, MockEnvParams};

use super::helpers::{self, mock_config, mock_routes};
use crate::contract::entry::{execute, instantiate};
use crate::msg::ExecuteMsg;
use crate::OsmosisRoute;

#[test]
fn instantiating() {
    let mut deps = helpers::setup_test();

    // config should have been correctly stored
    let cfg: Config<String> = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg, mock_config().into());

    // init config with safety_tax_rate greater than 1; should fail
    let mut cfg = mock_config();
    cfg.safety_tax_rate = Decimal::percent(150);

    let info = mock_info("deployer");
    let err = instantiate(deps.as_mut(), mock_env(), info, cfg.into()).unwrap_err();
    assert_eq!(
        err,
        ContractError::Mars(MarsError::InvalidParam {
            param_name: "safety_tax_rate".to_string(),
            invalid_value: "1.5".to_string(),
            predicate: "<= 1".to_string(),
        })
    );
}

#[test]
fn updating_config() {
    let mut deps = helpers::setup_test();

    let new_cfg = CreateOrUpdateConfig {
        safety_tax_rate: Some(Decimal::percent(69)),
        ..Default::default()
    };

    // non-owner is not authorized
    let info = mock_info("jake");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: new_cfg.clone(),
    };
    let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    assert_eq!(err, MarsError::Unauthorized {}.into());

    // update config with safety_tax_rate greater than 1
    let mut invalid_cfg = new_cfg.clone();
    invalid_cfg.safety_tax_rate = Some(Decimal::percent(125));

    let info = mock_info("owner");
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg: invalid_cfg,
    };
    let err = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::Mars(MarsError::InvalidParam {
            param_name: "safety_tax_rate".to_string(),
            invalid_value: "1.25".to_string(),
            predicate: "<= 1".to_string(),
        })
    );

    // update config properly
    let msg = ExecuteMsg::UpdateConfig {
        new_cfg,
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let cfg: Config<String> = helpers::query(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(cfg.safety_tax_rate, Decimal::percent(69));
}

#[test]
fn setting_route() {
    let mut deps = helpers::setup_test();

    let steps = vec![
        Step {
            pool_id: 1,
            denom_out: "uosmo".to_string(),
        },
        Step {
            pool_id: 420,
            denom_out: "umars".to_string(),
        },
    ];

    let msg = ExecuteMsg::SetRoute {
        denom_in: "uatom".to_string(),
        denom_out: "umars".to_string(),
        route: OsmosisRoute(steps.clone()),
    };
    let invalid_msg = ExecuteMsg::SetRoute {
        denom_in: "uatom".to_string(),
        denom_out: "umars".to_string(),
        route: OsmosisRoute(vec![]),
    };

    // non-owner is not authorized
    let err = execute(deps.as_mut(), mock_env(), mock_info("jake"), msg.clone()).unwrap_err();
    assert_eq!(err, MarsError::Unauthorized {}.into());

    // attempting to set an invalid swap route; should fail
    let err = execute(deps.as_mut(), mock_env(), mock_info("owner"), invalid_msg).unwrap_err();
    assert_eq!(
        err,
        ContractError::InvalidRoute {
            reason: "the route must contain at least one step".to_string()
        }
    );

    // properly set up route
    execute(deps.as_mut(), mock_env(), mock_info("owner"), msg).unwrap();

    let res: RouteResponse<OsmosisRoute> = helpers::query(
        deps.as_ref(),
        QueryMsg::Route {
            denom_in: "uatom".to_string(),
            denom_out: "umars".to_string(),
        },
    );
    assert_eq!(res.route, OsmosisRoute(steps));
}

#[test]
fn withdrawing_from_red_bank() {
    let mut deps = helpers::setup_test();

    // anyone can execute a withdrawal
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::WithdrawFromRedBank {
            denom: "uatom".to_string(),
            amount: Some(Uint128::new(42069)),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "red_bank".to_string(),
            msg: to_binary(&mars_outpost::red_bank::msg::ExecuteMsg::Withdraw {
                asset: mars_outpost::asset::Asset::Native {
                    denom: "uatom".to_string()
                },
                amount: Some(Uint128::new(42069)),
                recipient: None
            })
            .unwrap(),
            funds: vec![]
        }))
    )
}

#[test]
fn swapping_asset() {
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

#[test]
fn distributing_rewards() {
    let mut deps = helpers::setup_test();

    let env = mock_env_at_height_and_time(MockEnvParams {
        block_height: 10000,
        block_time: Timestamp::from_seconds(17000000),
    });

    // distribute uusdc to safety fund
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info("jake"),
        ExecuteMsg::DistributeRewards {
            denom: "uusdc".to_string(),
            amount: Some(Uint128::new(123)),
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: "channel-69".to_string(),
            to_address: "safety_fund".to_string(),
            amount: coin(123, "uusdc"),
            timeout: IbcTimeout::with_both(
                IbcTimeoutBlock {
                    revision: 1,
                    height: 10050,
                },
                Timestamp::from_seconds(17000300)
            )
        }))
    );

    // distribute umars to fee collector
    let res = execute(
        deps.as_mut(),
        env,
        mock_info("jake"),
        ExecuteMsg::DistributeRewards {
            denom: "umars".to_string(),
            amount: None,
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: "channel-69".to_string(),
            to_address: "fee_collector".to_string(),
            amount: coin(8964, "umars"),
            timeout: IbcTimeout::with_both(
                IbcTimeoutBlock {
                    revision: 1,
                    height: 10050,
                },
                Timestamp::from_seconds(17000300)
            )
        }))
    );

    // distribute uatom; should fail
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::DistributeRewards {
            denom: "uatom".to_string(),
            amount: Some(Uint128::new(123)),
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::AssetNotEnabledForDistribution {
            denom: "uatom".to_string()
        }
    );
}

#[test]
fn executing_cosmos_msg() {
    let mut deps = helpers::setup_test();

    let cosmos_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: "destination".to_string(),
        amount: vec![Coin {
            denom: "uluna".to_string(),
            amount: Uint128::new(123456),
        }],
    });
    let msg = ExecuteMsg::ExecuteCosmosMsg {
        cosmos_msg: cosmos_msg.clone(),
    };

    // non-owner is not authorized
    let err = execute(deps.as_mut(), mock_env(), mock_info("jake"), msg.clone()).unwrap_err();
    assert_eq!(err, MarsError::Unauthorized {}.into());

    // owner can execute cosmos msg
    let res = execute(deps.as_mut(), mock_env(), mock_info("owner"), msg).unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(res.messages[0], SubMsg::new(cosmos_msg));
}

#[test]
fn querying_routess() {
    let deps = helpers::setup_test();

    // NOTE: the response is ordered alphabetically
    let routes = mock_routes();
    let expected = vec![
        RouteResponse {
            denom_in: "uatom".to_string(),
            denom_out: "umars".to_string(),
            route: routes.get(&("uatom", "umars")).unwrap().clone(),
        },
        RouteResponse {
            denom_in: "uatom".to_string(),
            denom_out: "uusdc".to_string(),
            route: routes.get(&("uatom", "uusdc")).unwrap().clone(),
        },
        RouteResponse {
            denom_in: "uosmo".to_string(),
            denom_out: "umars".to_string(),
            route: routes.get(&("uosmo", "umars")).unwrap().clone(),
        },
    ];

    let res: Vec<RouteResponse<OsmosisRoute>> = helpers::query(
        deps.as_ref(),
        QueryMsg::Routes {
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(res, expected);

    let res: Vec<RouteResponse<OsmosisRoute>> = helpers::query(
        deps.as_ref(),
        QueryMsg::Routes {
            start_after: None,
            limit: Some(1),
        },
    );
    assert_eq!(res, expected[..1]);

    let res: Vec<RouteResponse<OsmosisRoute>> = helpers::query(
        deps.as_ref(),
        QueryMsg::Routes {
            start_after: Some(("uatom".to_string(), "uosmo".to_string())),
            limit: None,
        },
    );
    assert_eq!(res, expected[1..]);
}

#[test]
fn validating_route() {
    let deps = helpers::setup_test();
    let q = &deps.as_ref().querier;

    // invalid - route is empty
    let route = OsmosisRoute(vec![]);
    assert_eq!(
        route.validate(q, "uatom", "umars"),
        Err(ContractError::InvalidRoute {
            reason: "the route must contain at least one step".to_string()
        })
    );

    // invalid - the pool must contain the input denom
    let route = OsmosisRoute(vec![
        Step {
            pool_id: 68,
            denom_out: "uusdc".to_string(),
        },
        Step {
            pool_id: 420,
            denom_out: "umars".to_string(), // 420 is OSMO-MARS pool; but the previous step's output is USDC
        },
    ]);
    assert_eq!(
        route.validate(q, "uatom", "umars"),
        Err(ContractError::InvalidRoute {
            reason: "step 2: pool 420 does not contain input denom uusdc".to_string()
        })
    );

    // invalid - the pool must contain the output denom
    let route = OsmosisRoute(vec![
        Step {
            pool_id: 1,
            denom_out: "uosmo".to_string(),
        },
        Step {
            pool_id: 69,
            denom_out: "umars".to_string(), // 69 is OSMO-USDC pool; but this step's output is MARS
        },
    ]);
    assert_eq!(
        route.validate(q, "uatom", "umars"),
        Err(ContractError::InvalidRoute {
            reason: "step 2: pool 69 does not contain output denom umars".to_string()
        })
    );

    // invalid - route contains a loop
    // this examle: ATOM -> OSMO -> USDC -> OSMO -> MARS
    let route = OsmosisRoute(vec![
        Step {
            pool_id: 1,
            denom_out: "uosmo".to_string(),
        },
        Step {
            pool_id: 69,
            denom_out: "uusdc".to_string(),
        },
        Step {
            pool_id: 69,
            denom_out: "uosmo".to_string(),
        },
        Step {
            pool_id: 420,
            denom_out: "umars".to_string(),
        },
    ]);
    assert_eq!(
        route.validate(q, "uatom", "umars"),
        Err(ContractError::InvalidRoute {
            reason: "route contains a loop: denom uosmo seen twice".to_string()
        })
    );

    // invalid - route's final output denom does not match the desired output
    let route = OsmosisRoute(vec![
        Step {
            pool_id: 1,
            denom_out: "uosmo".to_string(),
        },
        Step {
            pool_id: 69,
            denom_out: "uusdc".to_string(),
        },
    ]);
    assert_eq!(
        route.validate(q, "uatom", "umars"),
        Err(ContractError::InvalidRoute {
            reason: "the route's output denom uusdc does not match the desired output umars"
                .to_string()
        })
    );

    // valid
    let route = OsmosisRoute(vec![
        Step {
            pool_id: 1,
            denom_out: "uosmo".to_string(),
        },
        Step {
            pool_id: 420,
            denom_out: "umars".to_string(),
        },
    ]);
    assert_eq!(route.validate(q, "uatom", "umars"), Ok(()));
}

#[test]
fn stringifying_route() {
    let route = OsmosisRoute(vec![
        Step {
            pool_id: 1,
            denom_out: "uosmo".to_string(),
        },
        Step {
            pool_id: 420,
            denom_out: "umars".to_string(),
        },
    ]);
    assert_eq!(route.to_string(), "1:uosmo|420:umars".to_string());
}

#[test]
fn unwrapping_option_amount() {
    let deps = helpers::setup_test();

    assert_eq!(
        unwrap_option_amount(
            &deps.as_ref().querier,
            &Addr::unchecked(MOCK_CONTRACT_ADDR),
            "uatom",
            None
        ),
        Ok(Uint128::new(88888))
    );
    assert_eq!(
        unwrap_option_amount(
            &deps.as_ref().querier,
            &Addr::unchecked(MOCK_CONTRACT_ADDR),
            "uatom",
            Some(Uint128::new(12345))
        ),
        Ok(Uint128::new(12345))
    );
    assert_eq!(
        unwrap_option_amount(
            &deps.as_ref().querier,
            &Addr::unchecked(MOCK_CONTRACT_ADDR),
            "uatom",
            Some(Uint128::new(99999))
        ),
        Err(ContractError::AmountToDistributeTooLarge {
            amount: Uint128::new(99999),
            balance: Uint128::new(88888),
        })
    );
}

#[test]
fn stringifying_option_amount() {
    assert_eq!(stringify_option_amount(Some(Uint128::new(42069))), "42069".to_string());
    assert_eq!(stringify_option_amount(None), "undefined".to_string());
}
