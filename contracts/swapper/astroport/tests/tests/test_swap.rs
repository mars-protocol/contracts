use astroport::{asset::AssetInfo, factory::PairType, pair::StablePoolParams};
use cosmwasm_std::{coin, to_json_binary, Binary, Decimal, Uint128};
use cw_it::{
    astroport::robot::AstroportTestRobot, robot::TestRobot, test_tube::Account, traits::CwItRunner,
};
use mars_oracle_wasm::WasmPriceSourceUnchecked;
use mars_swapper_astroport::config::AstroportConfig;
use mars_testing::{astroport_swapper::AstroportSwapperRobot, test_runner::get_test_runner};
use mars_types::swapper::SwapperRoute;
use test_case::test_case;

#[derive(Clone, Debug)]
enum PoolType {
    Xyk,
    Stable {
        amp: u64,
    },
}

impl From<PoolType> for PairType {
    fn from(pool_type: PoolType) -> Self {
        match pool_type {
            PoolType::Xyk => PairType::Xyk {},
            PoolType::Stable {
                ..
            } => PairType::Stable {},
        }
    }
}

impl PoolType {
    fn init_params(&self) -> Option<Binary> {
        match self {
            PoolType::Xyk => None,
            PoolType::Stable {
                amp,
            } => Some(
                to_json_binary(&StablePoolParams {
                    amp: *amp,
                    owner: None,
                })
                .unwrap(),
            ),
        }
    }
}

/// 1:1 ratio
const DEFAULT_LIQ: [u128; 2] = [10000000000000000u128, 10000000000000000u128];

#[test_case(PoolType::Xyk {}, "uatom", &DEFAULT_LIQ, &[6,6], Decimal::percent(0), false; "1:1 price and decimals, 0% slippage tolerance")]
#[test_case(PoolType::Xyk {}, "uatom", &DEFAULT_LIQ, &[6,6], Decimal::percent(0), true => panics ; "no route")]
#[test_case(PoolType::Stable { amp: 10u64 }, "uatom", &DEFAULT_LIQ, &[6,6], Decimal::percent(0), false; "stable swap 1:1 price and decimals, 0% slippage tolerance")]
#[test_case(PoolType::Stable { amp: 10u64 }, "uatom", &DEFAULT_LIQ, &[6,6], Decimal::percent(5), true => panics ; "stable swap no route")]
#[test_case(PoolType::Xyk {}, "uatom",  &DEFAULT_LIQ, &[10,6], Decimal::percent(1), false; "xyk 10:6 decimals, even pool")]
#[test_case(PoolType::Xyk {}, "uatom",  &DEFAULT_LIQ, &[6,18], Decimal::percent(1), false; "xyk 6:18 decimals, even pool")]
#[test_case(PoolType::Stable { amp: 10u64 }, "uatom", &[100000000000,10000000000000], &[6,8], Decimal::percent(10), false; "stable 6:8 decimals, even adjusted pool")]
#[test_case(PoolType::Stable { amp: 10u64 }, "uatom", &[1000000000000,100000000000], &[7,6], Decimal::percent(10), false; "stable 8:6 decimals, even adjusted pool")]
#[test_case(PoolType::Stable { amp: 10u64 }, "uatom", &[100000000000,100000000000000000000000], &[6,18], Decimal::percent(5), false; "stable 6:18 decimals, even adjusted pool")]
#[test_case(PoolType::Xyk {}, "uatom", &DEFAULT_LIQ, &[6,6], Decimal::percent(11), false => panics ; "xyk max slippage exceeded")]
#[test_case(PoolType::Stable { amp: 10u64 }, "uatom", &DEFAULT_LIQ, &[6,6], Decimal::percent(11), false => panics ; "stable max slippage exceeded")]
fn swap(
    pool_type: PoolType,
    denom_out: &str,
    pool_liq: &[u128; 2],
    decimals: &[u8; 2],
    slippage: Decimal,
    no_route: bool,
) {
    let denom_in = "uosmo";
    let swaps = vec![mars_types::swapper::AstroSwap {
        from: denom_in.to_string(),
        to: denom_out.to_string(),
    }];
    let coin_in = coin(1000000, denom_in);

    let owned_runner = get_test_runner();
    let runner = owned_runner.as_ref();
    let initial_balance = Uint128::from(10000000000000000000000000u128);
    let admin = runner
        .init_account(&[
            coin(initial_balance.u128(), denom_in),
            coin(initial_balance.u128(), denom_out),
        ])
        .unwrap();
    let alice = runner
        .init_account(&[
            coin(initial_balance.u128(), denom_in),
            coin(initial_balance.u128(), denom_out),
        ])
        .unwrap();
    let robot = AstroportSwapperRobot::new_with_local(&runner, &admin);

    // Create astroport pair for uosmo/usd
    let (pair_address, _lp_token_addr) = robot.create_astroport_pair(
        pool_type.clone().into(),
        &[
            AssetInfo::NativeToken {
                denom: denom_in.to_string(),
            },
            AssetInfo::NativeToken {
                denom: denom_out.to_string(),
            },
        ],
        pool_type.init_params(),
        &admin,
        Some(pool_liq),
        Some(decimals),
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
            },
            &admin,
        );

    robot.set_config(
        AstroportConfig {
            router: robot.astroport_contracts().router.address.clone(),
            factory: robot.astroport_contracts().factory.address.clone(),
            oracle: robot.oracle_robot.mars_oracle_contract_addr.clone(),
        },
        &admin,
    );

    let swaps = if !no_route {
        swaps
    } else {
        vec![]
    };

    let route = SwapperRoute::Astro(mars_types::swapper::AstroRoute {
        swaps,
    });

    let estimated_amount = robot.query_estimate_exact_in_swap(&coin_in, denom_out, route.clone());

    println!("Estimated amount: {}", estimated_amount);

    let balance = robot
        .swap(coin_in, denom_out, slippage, &alice, route)
        .query_native_token_balance(alice.address(), denom_out);

    let received_amount = balance - initial_balance;

    println!("Received amount: {}", received_amount);

    assert!((received_amount) >= slippage * estimated_amount);
}
