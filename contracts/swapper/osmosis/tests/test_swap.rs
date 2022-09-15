use cosmwasm_std::{coin, Addr, Decimal, Querier, QuerierResult, QuerierWrapper, Uint128};
use cw_multi_test::Executor;
use osmo_bindings::Step;
use osmo_bindings_test::{OsmosisApp, OsmosisError, Pool};

use rover::adapters::swap::ExecuteMsg;
use rover::error::ContractError as RoverError;
use swapper_base::ContractError;
use swapper_base::Route;
use swapper_osmosis::route::OsmosisRoute;

use crate::helpers::mock_osmosis_app;
use crate::helpers::{assert_err, instantiate_contract};

pub mod helpers;

#[test]
fn test_transfer_callback_only_internal() {
    let mut app = mock_osmosis_app();
    let contract_addr = instantiate_contract(&mut app);

    let bad_guy = Addr::unchecked("bad_guy");
    let res = app.execute_contract(
        bad_guy.clone(),
        contract_addr,
        &ExecuteMsg::<OsmosisRoute>::TransferResult {
            recipient: bad_guy.clone(),
            denom_in: "mars".to_string(),
            denom_out: "osmo".to_string(),
        },
        &[],
    );

    assert_err(
        res,
        ContractError::Rover(RoverError::Unauthorized {
            user: bad_guy.to_string(),
            action: "transfer result".to_string(),
        }),
    );
}

#[test]
fn test_swap_exact_in_slippage_too_high() {
    pub struct MockQuerier<'a> {
        app: &'a OsmosisApp,
    }
    impl Querier for MockQuerier<'_> {
        fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
            self.app.raw_query(bin_request)
        }
    }

    let owner = Addr::unchecked("owner");
    let whale = Addr::unchecked("whale");
    let mut app = mock_osmosis_app();
    let contract_addr = instantiate_contract(&mut app);

    let coin_a = coin(6_000_000, "mars");
    let coin_b = coin(1_500_000, "osmo");
    let pool_id_x = 43;
    let pool_x = Pool::new(coin_a, coin_b.clone());

    app.init_modules(|router, _, storage| {
        router.custom.set_pool(storage, pool_id_x, &pool_x).unwrap();
        router
            .bank
            .init_balance(storage, &owner, vec![coin(10_000, "mars")])
            .unwrap();
        router
            .bank
            .init_balance(storage, &whale, vec![coin(1_000_000, "mars")])
            .unwrap();
    });

    let route = OsmosisRoute {
        steps: vec![Step {
            pool_id: pool_id_x,
            denom_out: coin_b.denom,
        }],
    };

    app.execute_contract(
        owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::SetRoute {
            denom_in: "mars".to_string(),
            denom_out: "osmo".to_string(),
            route: route.clone(),
        },
        &[],
    )
    .unwrap();

    let querier = MockQuerier { app: &app };
    let mock_querier = QuerierWrapper::new(&querier);

    // Simulate real-time slippage by generating swapper message first, changing pool ratio, and then swapping with that message
    let msg = route
        .build_exact_in_swap_msg(
            &mock_querier,
            contract_addr.clone(),
            &coin(10_000, "mars"),
            Decimal::from_atomics(6u128, 2).unwrap(),
        )
        .unwrap();

    // whale does a huge trade to make mars less valuable
    app.execute_contract(
        whale,
        contract_addr,
        &ExecuteMsg::<OsmosisRoute>::SwapExactIn {
            coin_in: coin(1_000_000, "mars"),
            denom_out: "osmo".to_string(),
            slippage: Decimal::from_atomics(1u128, 0).unwrap(),
        },
        &[coin(1_000_000, "mars")],
    )
    .unwrap();

    // Resume initial user's trade but the output is less than slippage allowance
    let res = app.execute(owner, msg);

    let error: OsmosisError = res.unwrap_err().downcast().unwrap();
    assert_eq!(error, OsmosisError::PriceTooLow);
}

#[test]
fn test_swap_exact_in_success() {
    let owner = Addr::unchecked("owner");
    let mut app = mock_osmosis_app();
    let contract_addr = instantiate_contract(&mut app);

    let coin_a = coin(6_000_000, "mars");
    let coin_b = coin(1_500_000, "osmo");
    let pool_id_x = 43;
    let pool_x = Pool::new(coin_a, coin_b);

    app.init_modules(|router, _, storage| {
        router.custom.set_pool(storage, pool_id_x, &pool_x).unwrap();
        router
            .bank
            .init_balance(storage, &owner, vec![coin(10_000, "mars")])
            .unwrap();
    });

    app.execute_contract(
        owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::SetRoute {
            denom_in: "mars".to_string(),
            denom_out: "osmo".to_string(),
            route: OsmosisRoute {
                steps: vec![Step {
                    pool_id: pool_id_x,
                    denom_out: "osmo".to_string(),
                }],
            },
        },
        &[],
    )
    .unwrap();

    let mars_balance = app.wrap().query_balance(owner.to_string(), "mars").unwrap();
    let osmo_balance = app.wrap().query_balance(owner.to_string(), "osmo").unwrap();

    assert_eq!(mars_balance.amount, Uint128::new(10_000));
    assert_eq!(osmo_balance.amount, Uint128::zero());

    app.execute_contract(
        owner.clone(),
        contract_addr.clone(),
        &ExecuteMsg::<OsmosisRoute>::SwapExactIn {
            coin_in: coin(10_000, "mars"),
            denom_out: "osmo".to_string(),
            slippage: Decimal::from_atomics(6u128, 2).unwrap(),
        },
        &[coin(10_000, "mars")],
    )
    .unwrap();

    // Assert user receives their new tokens
    let mars_balance = app.wrap().query_balance(owner.to_string(), "mars").unwrap();
    let osmo_balance = app.wrap().query_balance(owner.to_string(), "osmo").unwrap();

    assert_eq!(mars_balance.amount, Uint128::zero());
    assert_eq!(osmo_balance.amount, Uint128::new(2489));

    // Assert no tokens in contract left over
    let mars_balance = app
        .wrap()
        .query_balance(contract_addr.to_string(), "mars")
        .unwrap();
    let osmo_balance = app
        .wrap()
        .query_balance(contract_addr.to_string(), "osmo")
        .unwrap();

    assert_eq!(mars_balance.amount, Uint128::zero());
    assert_eq!(osmo_balance.amount, Uint128::zero());
}
