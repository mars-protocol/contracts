use cosmwasm_std::testing::mock_env;
use mars_outpost::{
    error::MarsError,
    rewards_collector::{QueryMsg, RouteResponse},
};
use mars_owner::OwnerError::NotOwner;
use mars_rewards_collector_base::{ContractError, Route};
use mars_rewards_collector_osmosis::{contract::entry::execute, msg::ExecuteMsg, OsmosisRoute};
use mars_testing::mock_info;
use osmosis_std::types::osmosis::gamm::v1beta1::SwapAmountInRoute;

use crate::helpers::mock_routes;

mod helpers;

#[test]
fn setting_route() {
    let mut deps = helpers::setup_test();

    let steps = vec![
        SwapAmountInRoute {
            pool_id: 1,
            token_out_denom: "uosmo".to_string(),
        },
        SwapAmountInRoute {
            pool_id: 420,
            token_out_denom: "umars".to_string(),
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
    assert_eq!(err, ContractError::Owner(NotOwner {}));

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
fn denom_with_invalid_char() {
    let mut deps = helpers::setup_test();

    let steps = vec![
        SwapAmountInRoute {
            pool_id: 1,
            token_out_denom: "uosmo".to_string(),
        },
        SwapAmountInRoute {
            pool_id: 420,
            token_out_denom: "umars".to_string(),
        },
    ];

    let msg = ExecuteMsg::SetRoute {
        denom_in: "hadb%akdjb!".to_string(),
        denom_out: "askd&7ab12d&".to_string(),
        route: OsmosisRoute(steps),
    };

    let res = execute(deps.as_mut(), mock_env(), mock_info("owner"), msg);
    assert_eq!(
        res,
        Err(ContractError::Mars(MarsError::InvalidDenom {
            reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                .to_string()
        }))
    );
}

#[test]
fn invalid_denom_length() {
    let mut deps = helpers::setup_test();

    let steps = vec![
        SwapAmountInRoute {
            pool_id: 1,
            token_out_denom: "uosmo".to_string(),
        },
        SwapAmountInRoute {
            pool_id: 420,
            token_out_denom: "umars".to_string(),
        },
    ];

    let msg = ExecuteMsg::SetRoute {
        denom_in: "qw".to_string(),
        denom_out: "qwrouwetsdknfsljvnsdkjfhw".to_string(),
        route: OsmosisRoute(steps),
    };

    let res = execute(deps.as_mut(), mock_env(), mock_info("owner"), msg);
    assert_eq!(
        res,
        Err(ContractError::Mars(MarsError::InvalidDenom {
            reason: "Invalid denom length".to_string(),
        }))
    );
}

#[test]
fn querying_routes() {
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
        RouteResponse {
            denom_in: "uusdc".to_string(),
            denom_out: "umars".to_string(),
            route: routes.get(&("uusdc", "umars")).unwrap().clone(),
        }
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
        SwapAmountInRoute {
            pool_id: 68,
            token_out_denom: "uusdc".to_string(),
        },
        SwapAmountInRoute {
            pool_id: 420,
            token_out_denom: "umars".to_string(), // 420 is OSMO-MARS pool; but the previous step's output is USDC
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
        SwapAmountInRoute {
            pool_id: 1,
            token_out_denom: "uosmo".to_string(),
        },
        SwapAmountInRoute {
            pool_id: 69,
            token_out_denom: "umars".to_string(), // 69 is OSMO-USDC pool; but this step's output is MARS
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
        SwapAmountInRoute {
            pool_id: 1,
            token_out_denom: "uosmo".to_string(),
        },
        SwapAmountInRoute {
            pool_id: 69,
            token_out_denom: "uusdc".to_string(),
        },
        SwapAmountInRoute {
            pool_id: 69,
            token_out_denom: "uosmo".to_string(),
        },
        SwapAmountInRoute {
            pool_id: 420,
            token_out_denom: "umars".to_string(),
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
        SwapAmountInRoute {
            pool_id: 1,
            token_out_denom: "uosmo".to_string(),
        },
        SwapAmountInRoute {
            pool_id: 69,
            token_out_denom: "uusdc".to_string(),
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
        SwapAmountInRoute {
            pool_id: 1,
            token_out_denom: "uosmo".to_string(),
        },
        SwapAmountInRoute {
            pool_id: 420,
            token_out_denom: "umars".to_string(),
        },
    ]);
    assert_eq!(route.validate(q, "uatom", "umars"), Ok(()));
}

#[test]
fn stringifying_route() {
    let route = OsmosisRoute(vec![
        SwapAmountInRoute {
            pool_id: 1,
            token_out_denom: "uosmo".to_string(),
        },
        SwapAmountInRoute {
            pool_id: 420,
            token_out_denom: "umars".to_string(),
        },
    ]);
    assert_eq!(route.to_string(), "1:uosmo|420:umars".to_string());
}
