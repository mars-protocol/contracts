use astroport::{asset::AssetInfo, factory::PairType, router::SwapOperation};
use cosmwasm_std::{coin, Decimal, Uint128};
use cw_it::{
    astroport::robot::AstroportTestRobot, robot::TestRobot, test_tube::Account, traits::CwItRunner,
};
use mars_oracle_wasm::WasmPriceSourceUnchecked;
use mars_testing::astroport_swapper::{get_test_runner, AstroportSwapperRobot};
use test_case::test_case;

#[test_case("usd", Decimal::percent(5), false ; "5% slippage tolerance")]
#[test_case("usd", Decimal::percent(5), true => panics ; "no route")]
#[test_case("usd", Decimal::percent(0), false => panics ; "0% slippage tolerance")]
fn test_swap(denom_out: &str, slippage: Decimal, no_route: bool) {
    let denom_in = "uosmo";
    let operations = vec![SwapOperation::AstroSwap {
        offer_asset_info: AssetInfo::NativeToken {
            denom: denom_in.to_string(),
        },
        ask_asset_info: AssetInfo::NativeToken {
            denom: denom_out.to_string(),
        },
    }];
    let coin_in = coin(1000000, denom_in);

    let runner = get_test_runner();
    let admin = runner
        .init_account(&[coin(1000000000000, denom_in), coin(10000000000000, denom_out)])
        .unwrap();
    let alice = runner
        .init_account(&[coin(1000000000000, denom_in), coin(10000000000000, denom_out)])
        .unwrap();
    let robot = AstroportSwapperRobot::new_with_local(&runner, &admin);

    // Create astropor pair for uosmo/usd
    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        PairType::Xyk {},
        [
            AssetInfo::NativeToken {
                denom: denom_in.to_string(),
            },
            AssetInfo::NativeToken {
                denom: denom_out.to_string(),
            },
        ],
        None,
        &admin,
        Some([10000000000u128.into(), 10000000000u128.into()]),
    );

    // Setup oracle prices
    robot
        .oracle_robot
        .set_price_source(
            denom_out,
            WasmPriceSourceUnchecked::Fixed {
                price: Decimal::one(),
            },
            &admin,
        )
        .set_price_source(
            denom_in,
            WasmPriceSourceUnchecked::AstroportSpot {
                pair_address,
                route_assets: vec![],
            },
            &admin,
        );

    robot
        .add_denom_precision_to_coin_registry(denom_in, 6, &admin)
        .add_denom_precision_to_coin_registry(denom_out, 6, &admin);

    if !no_route {
        robot.set_route(operations, denom_in, denom_out, &admin);
    }

    let estimated_amount = robot.query_estimate_exact_in_swap(&coin_in, denom_out);

    let balance = robot
        .swap(coin_in, denom_out, slippage, &alice)
        .query_native_token_balance(alice.address(), denom_out);

    assert!((balance - Uint128::new(10000000000000u128)) >= slippage * estimated_amount);
}
