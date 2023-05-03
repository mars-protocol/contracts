use astroport::{asset::AssetInfo, router::SwapOperation};
use cosmwasm_std::coin;
use cw_it::traits::CwItRunner;

use mars_testing::astroport_swapper::{get_test_runner, AstroportSwapperRobot};
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
fn test_set_route(denom_in: &str, denom_out: &str, operations: Vec<SwapOperation>) {
    let runner = get_test_runner();
    let admin = runner.init_account(&[coin(1000000000000, "uosmo")]).unwrap();
    let robot = AstroportSwapperRobot::new_with_local(&runner, &admin);

    robot.set_route(operations, denom_in, denom_out, &admin);
}

#[test]
#[should_panic]
fn test_set_route_not_admin() {
    let runner = get_test_runner();
    let admin = runner.init_account(&[coin(1000000000000, "uosmo")]).unwrap();
    let caller = runner.init_account(&[coin(1000000000000, "uosmo")]).unwrap();
    let robot = AstroportSwapperRobot::new_with_local(&runner, &admin);

    let denom_in = "uosmo";
    let denom_out = "uusd";
    let operations = to_native_swap_operations(vec![(denom_in, denom_out)]);

    robot.set_route(operations, denom_in, denom_out, &caller);
}
