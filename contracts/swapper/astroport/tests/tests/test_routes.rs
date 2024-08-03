use astroport_v5::{asset::AssetInfo, router::SwapOperation};
use cosmwasm_std::coin;
use cw_it::{astroport::robot::AstroportTestRobot, robot::TestRobot, traits::CwItRunner};
use mars_swapper_astroport::route::AstroportRoute;
use mars_testing::{astroport_swapper::AstroportSwapperRobot, test_runner::get_test_runner};
use mars_types::swapper::RouteResponse;
use test_case::test_case;

fn to_native_swap_operation((denom_in, denom_out): (&str, &str)) -> SwapOperation {
    SwapOperation::AstroSwap {
        offer_asset_info: AssetInfo::NativeToken {
            denom: denom_in.to_string(),
        },
        ask_asset_info: AssetInfo::NativeToken {
            denom: denom_out.to_string(),
        },
    }
}

fn to_native_swap_operations(operations: Vec<(&str, &str)>) -> Vec<SwapOperation> {
    operations.into_iter().map(to_native_swap_operation).collect()
}

#[test]
fn test_to_native_swap_operations() {
    let operations = to_native_swap_operations(vec![("uusd", "uosmo")]);
    assert_eq!(
        operations,
        vec![SwapOperation::AstroSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string()
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "uosmo".to_string()
            }
        }]
    );
}

#[test]
fn test_to_native_swap_operation() {
    let operation = to_native_swap_operation(("uusd", "uosmo"));
    assert_eq!(
        operation,
        SwapOperation::AstroSwap {
            offer_asset_info: AssetInfo::NativeToken {
                denom: "uusd".to_string()
            },
            ask_asset_info: AssetInfo::NativeToken {
                denom: "uosmo".to_string()
            }
        }
    );
}

#[test_case("uosmo", "uusd", to_native_swap_operations(vec![("uosmo", "uatom"), ("uatom", "uusd")]) ; "correct route")]
#[test_case("uosmo", "uusd", vec![] => panics ; "no operations")]
#[test_case("uosmo", "uusd", to_native_swap_operations(vec![("uatom", "uusd")]) => panics ; "first route step does not contain input denom")]
#[test_case("uosmo", "uusd", to_native_swap_operations(vec![("uosmo", "uatom")]) => panics ; "last route step does not contain input denom")]
#[test_case("uosmo", "uusd", to_native_swap_operations(vec![("uosmo", "uatom"), ("uatom", "uusd"), ("uusd", "uosmo")]) => panics ; "route contains cycle")]
#[test_case("uosmo", "uusd", to_native_swap_operations(vec![("uosmo", "uatom"), ("uusd", "ustrd")]) => panics ; "route is not connected")]
#[test_case("uosmo", "uusd", vec![SwapOperation::NativeSwap { offer_denom: "uosmo".to_string(), ask_denom: "uusd".to_string() }] => panics ; "contains NativeSwap operation")]
fn set_route(denom_in: &str, denom_out: &str, operations: Vec<SwapOperation>) {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = runner.init_account(&[coin(1000000000000, "uosmo")]).unwrap();
    let robot = AstroportSwapperRobot::new_with_local(&runner, &admin);

    robot
        .set_route(operations.clone(), denom_in, denom_out, &admin)
        .assert_route(denom_in, denom_out, operations);
}

#[test]
#[should_panic]
fn set_route_not_admin() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = runner.init_account(&[coin(1000000000000, "uosmo")]).unwrap();
    let caller = runner.init_account(&[coin(1000000000000, "uosmo")]).unwrap();
    let robot = AstroportSwapperRobot::new_with_local(&runner, &admin);

    let denom_in = "uosmo";
    let denom_out = "uusd";
    let operations = to_native_swap_operations(vec![(denom_in, denom_out)]);

    robot.set_route(operations, denom_in, denom_out, &caller);
}

#[test]
fn query_non_existing_route() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = runner.init_account(&[coin(1000000000000, "uosmo")]).unwrap();
    let robot = AstroportSwapperRobot::new_with_local(&runner, &admin);

    let denom_in = "uosmo";
    let denom_out = "uusd";

    let err = robot
        .wasm()
        .query::<_, RouteResponse<AstroportRoute>>(
            &robot.swapper,
            &mars_types::swapper::QueryMsg::Route {
                denom_in: denom_in.into(),
                denom_out: denom_out.into(),
            },
        )
        .unwrap_err()
        .to_string();

    assert!(err.contains("No route found from uosmo to uusd"));
}

#[test]
fn query_routes() {
    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let admin = runner.init_account(&[coin(1000000000000, "uosmo")]).unwrap();
    let robot = AstroportSwapperRobot::new_with_local(&runner, &admin);

    let denom_in = "uosmo";
    let denom_out = "uusd";
    let operations_1 = to_native_swap_operations(vec![(denom_in, denom_out)]);
    let operations_2 = to_native_swap_operations(vec![(denom_out, denom_in)]);

    robot.set_route(operations_1.clone(), denom_in, denom_out, &admin).set_route(
        operations_2.clone(),
        denom_out,
        denom_in,
        &admin,
    );

    let routes = robot.query_routes(None, None);

    assert_eq!(routes.len(), 2);
    assert!(routes.contains(&RouteResponse {
        denom_in: denom_in.to_string(),
        denom_out: denom_out.to_string(),
        route: AstroportRoute {
            operations: operations_1,
            router: robot.astroport_contracts().router.address.clone(),
            factory: robot.astroport_contracts().factory.address.clone(),
            oracle: robot.oracle_robot.mars_oracle_contract_addr.clone(),
        },
    }));
    assert!(routes.contains(&RouteResponse {
        denom_in: denom_out.to_string(),
        denom_out: denom_in.to_string(),
        route: AstroportRoute {
            operations: operations_2,
            router: robot.astroport_contracts().router.address.clone(),
            factory: robot.astroport_contracts().factory.address.clone(),
            oracle: robot.oracle_robot.mars_oracle_contract_addr,
        },
    }));
}
