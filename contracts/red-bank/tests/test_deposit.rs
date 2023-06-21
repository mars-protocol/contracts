use std::any::type_name;

use cosmwasm_std::{
    attr, coin, coins,
    testing::{mock_env, mock_info, MockApi, MockStorage},
    to_binary, Addr, Decimal, OwnedDeps, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw_utils::PaymentError;
use helpers::{
    set_collateral, th_build_interests_updated_event, th_get_expected_indices_and_rates, th_setup,
};
use mars_params::types::{AssetParams, HighLeverageStrategyParams, RedBankSettings, RoverSettings};
use mars_red_bank::{
    contract::execute,
    error::ContractError,
    interest_rates::{compute_scaled_amount, ScalingOperation, SCALING_FACTOR},
    state::{COLLATERALS, MARKETS},
};
use mars_red_bank_types::{
    address_provider::MarsAddressType,
    incentives,
    red_bank::{Collateral, ExecuteMsg, Market},
};
use mars_testing::{mock_env_at_block_time, MarsMockQuerier};

use crate::helpers::th_default_asset_params;

mod helpers;

struct TestSuite {
    deps: OwnedDeps<MockStorage, MockApi, MarsMockQuerier>,
    denom: &'static str,
    depositor_addr: Addr,
    initial_market: Market,
}

fn setup_test() -> TestSuite {
    let denom = "uosmo";
    let initial_liquidity = Uint128::new(10_000_000);

    let mut deps = th_setup(&[coin(initial_liquidity.u128(), denom)]);

    let market = Market {
        denom: denom.to_string(),
        liquidity_index: Decimal::from_ratio(11u128, 10u128),
        borrow_index: Decimal::from_ratio(1u128, 1u128),
        borrow_rate: Decimal::from_ratio(10u128, 100u128),
        liquidity_rate: Decimal::from_ratio(10u128, 100u128),
        reserve_factor: Decimal::from_ratio(1u128, 10u128),
        collateral_total_scaled: Uint128::new(10_000_000) * SCALING_FACTOR,
        debt_total_scaled: Uint128::new(10_000_000) * SCALING_FACTOR,
        indexes_last_updated: 10000000,
        ..Default::default()
    };

    MARKETS.save(deps.as_mut().storage, denom, &market).unwrap();

    deps.querier.set_redbank_params(
        denom,
        AssetParams {
            max_loan_to_value: Decimal::one(),
            liquidation_threshold: Default::default(),
            liquidation_bonus: Default::default(),
            rover: RoverSettings {
                whitelisted: false,
                hls: HighLeverageStrategyParams {
                    max_loan_to_value: Decimal::percent(90),
                    liquidation_threshold: Decimal::one(),
                },
            },
            red_bank: RedBankSettings {
                deposit_enabled: true,
                borrow_enabled: true,
                deposit_cap: Uint128::new(12_000_000),
            },
        },
    );

    TestSuite {
        deps,
        denom,
        depositor_addr: Addr::unchecked("larry"),
        initial_market: market,
    }
}

#[test]
fn depositing_with_no_coin_sent() {
    let TestSuite {
        mut deps,
        depositor_addr,
        ..
    } = setup_test();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(depositor_addr.as_str(), &[]),
        ExecuteMsg::Deposit {
            on_behalf_of: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, PaymentError::NoFunds {}.into());
}

#[test]
fn depositing_with_multiple_coins_sent() {
    let TestSuite {
        mut deps,
        depositor_addr,
        ..
    } = setup_test();

    let sent_coins = vec![coin(123, "uatom"), coin(456, "uosmo")];

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(depositor_addr.as_str(), &sent_coins),
        ExecuteMsg::Deposit {
            on_behalf_of: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, PaymentError::MultipleDenoms {}.into());
}

#[test]
fn depositing_to_non_existent_market() {
    let TestSuite {
        mut deps,
        depositor_addr,
        ..
    } = setup_test();

    // there isn't a market for this denom
    let false_denom = "usteak";

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(depositor_addr.as_str(), &coins(123, false_denom)),
        ExecuteMsg::Deposit {
            on_behalf_of: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, StdError::not_found(type_name::<Market>()).into());
}

#[test]
fn depositing_to_disabled_market() {
    let TestSuite {
        mut deps,
        denom,
        depositor_addr,
        ..
    } = setup_test();

    // disable the market
    deps.querier.set_redbank_params(
        denom,
        AssetParams {
            rover: RoverSettings {
                whitelisted: false,
                hls: HighLeverageStrategyParams {
                    max_loan_to_value: Decimal::percent(90),
                    liquidation_threshold: Decimal::one(),
                },
            },
            red_bank: RedBankSettings {
                deposit_enabled: false,
                borrow_enabled: true,
                deposit_cap: Default::default(),
            },
            ..th_default_asset_params()
        },
    );

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(depositor_addr.as_str(), &coins(123, denom)),
        ExecuteMsg::Deposit {
            on_behalf_of: None,
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::DepositNotEnabled {
            denom: denom.to_string(),
        }
    );
}

#[test]
fn depositing_above_cap() {
    let TestSuite {
        mut deps,
        denom,
        depositor_addr,
        ..
    } = setup_test();

    // set a deposit cap
    MARKETS
        .update(deps.as_mut().storage, denom, |opt| -> StdResult<_> {
            let mut market = opt.unwrap();
            market.collateral_total_scaled = Uint128::new(9_000_000) * SCALING_FACTOR;
            Ok(market)
        })
        .unwrap();
    deps.querier.set_redbank_params(
        denom,
        AssetParams {
            rover: RoverSettings {
                whitelisted: false,
                hls: HighLeverageStrategyParams {
                    max_loan_to_value: Decimal::percent(90),
                    liquidation_threshold: Decimal::one(),
                },
            },
            red_bank: RedBankSettings {
                deposit_enabled: true,
                borrow_enabled: true,
                deposit_cap: Uint128::new(10_000_000),
            },
            ..th_default_asset_params()
        },
    );

    // try deposit with a big amount, should fail
    let err = execute(
        deps.as_mut(),
        mock_env_at_block_time(10000100),
        mock_info(depositor_addr.as_str(), &coins(1_000_001, denom)),
        ExecuteMsg::Deposit {
            on_behalf_of: None,
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::DepositCapExceeded {
            denom: denom.to_string()
        }
    );

    // deposit a smaller amount, should work
    let result = execute(
        deps.as_mut(),
        mock_env_at_block_time(10000100),
        mock_info(depositor_addr.as_str(), &coins(123, denom)),
        ExecuteMsg::Deposit {
            on_behalf_of: None,
        },
    );
    assert!(result.is_ok());
}

#[test]
fn depositing_without_existing_position() {
    let TestSuite {
        mut deps,
        denom,
        depositor_addr,
        initial_market,
    } = setup_test();

    let block_time = 10000100;
    let deposit_amount = 110000;

    // compute expected market parameters
    let expected_params =
        th_get_expected_indices_and_rates(&initial_market, block_time, Default::default());
    let expected_mint_amount = compute_scaled_amount(
        Uint128::from(deposit_amount),
        expected_params.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_block_time(block_time),
        mock_info(depositor_addr.as_str(), &coins(deposit_amount, denom)),
        ExecuteMsg::Deposit {
            on_behalf_of: None,
        },
    )
    .unwrap();

    // NOTE: For this particular test, the borrow interest accrued was so low that the accrued
    // protocol reward is rounded down to zero. Therefore we don't expect a message to update the
    // index of the reward collector.
    assert_eq!(
        res.messages,
        vec![SubMsg::new(WasmMsg::Execute {
            contract_addr: MarsAddressType::Incentives.to_string(),
            msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                user_addr: depositor_addr.clone(),
                denom: initial_market.denom.clone(),
                user_amount_scaled_before: Uint128::zero(),
                // NOTE: Protocol rewards accrued is zero, so here it's initial total supply
                total_amount_scaled_before: initial_market.collateral_total_scaled,
            })
            .unwrap(),
            funds: vec![]
        })]
    );
    assert_eq!(
        res.attributes,
        vec![
            attr("action", "deposit"),
            attr("sender", &depositor_addr),
            attr("on_behalf_of", &depositor_addr),
            attr("denom", denom),
            attr("amount", deposit_amount.to_string()),
            attr("amount_scaled", expected_mint_amount),
        ]
    );
    assert_eq!(res.events, vec![th_build_interests_updated_event(denom, &expected_params)]);

    // indexes and interest rates should have been updated
    let market = MARKETS.load(deps.as_ref().storage, denom).unwrap();
    assert_eq!(market.borrow_index, expected_params.borrow_index);
    assert_eq!(market.liquidity_index, expected_params.liquidity_index);
    assert_eq!(market.borrow_rate, expected_params.borrow_rate);
    assert_eq!(market.liquidity_rate, expected_params.liquidity_rate);

    // total collateral amount should have been updated
    let expected = initial_market.collateral_total_scaled + expected_mint_amount;
    assert_eq!(market.collateral_total_scaled, expected);

    // the depositor previously did not have a collateral position
    // a position should have been created with the correct scaled amount, and enabled by default
    let collateral = COLLATERALS.load(deps.as_ref().storage, (&depositor_addr, denom)).unwrap();
    assert_eq!(
        collateral,
        Collateral {
            amount_scaled: expected_mint_amount,
            enabled: true
        }
    );
}

#[test]
fn depositing_with_existing_position() {
    let TestSuite {
        mut deps,
        denom,
        depositor_addr,
        initial_market,
    } = setup_test();

    // create a collateral position for the user, with the `enabled` parameter as false
    let collateral_amount_scaled = Uint128::new(123456);
    set_collateral(deps.as_mut(), &depositor_addr, denom, collateral_amount_scaled, false);

    let block_time = 10000100;
    let deposit_amount = 110000;

    // compute expected market parameters
    let expected_params =
        th_get_expected_indices_and_rates(&initial_market, block_time, Default::default());
    let expected_mint_amount = compute_scaled_amount(
        Uint128::from(deposit_amount),
        expected_params.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_block_time(block_time),
        mock_info(depositor_addr.as_str(), &coins(deposit_amount, denom)),
        ExecuteMsg::Deposit {
            on_behalf_of: None,
        },
    )
    .unwrap();

    // NOTE: For this particular test, the borrow interest accrued was so low that the accrued
    // protocol reward is rounded down to zero. Therefore we don't expect a message to update the
    // index of the reward collector.
    assert_eq!(
        res.messages,
        vec![SubMsg::new(WasmMsg::Execute {
            contract_addr: MarsAddressType::Incentives.to_string(),
            msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                user_addr: depositor_addr.clone(),
                denom: initial_market.denom.clone(),
                user_amount_scaled_before: collateral_amount_scaled,
                // NOTE: Protocol rewards accrued is zero, so here it's initial total supply
                total_amount_scaled_before: initial_market.collateral_total_scaled,
            })
            .unwrap(),
            funds: vec![]
        })]
    );

    // the depositor's scaled collateral amount should have been increased
    // however, the `enabled` status should not been affected
    let collateral = COLLATERALS.load(deps.as_ref().storage, (&depositor_addr, denom)).unwrap();
    let expected = collateral_amount_scaled + expected_mint_amount;
    assert_eq!(
        collateral,
        Collateral {
            amount_scaled: expected,
            enabled: false
        }
    );
}

#[test]
fn depositing_on_behalf_of() {
    let TestSuite {
        mut deps,
        denom,
        depositor_addr,
        initial_market,
    } = setup_test();

    let deposit_amount = 123456u128;
    let on_behalf_of_addr = Addr::unchecked("jake");

    // compute expected market parameters
    let block_time = 10000300;
    let expected_params =
        th_get_expected_indices_and_rates(&initial_market, block_time, Default::default());
    let expected_mint_amount = compute_scaled_amount(
        Uint128::from(deposit_amount),
        expected_params.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();
    let expected_reward_amount_scaled = compute_scaled_amount(
        expected_params.protocol_rewards_to_distribute,
        expected_params.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    let res = execute(
        deps.as_mut(),
        mock_env_at_block_time(block_time),
        mock_info(depositor_addr.as_str(), &coins(deposit_amount, denom)),
        ExecuteMsg::Deposit {
            on_behalf_of: Some(on_behalf_of_addr.clone().into()),
        },
    )
    .unwrap();

    // NOTE: For this test, the accrued protocol reward is non-zero, so we do expect a message to
    // update the index of the rewards collector.
    assert_eq!(
        res.messages,
        vec![
            SubMsg::new(WasmMsg::Execute {
                contract_addr: MarsAddressType::Incentives.to_string(),
                msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                    user_addr: Addr::unchecked(MarsAddressType::RewardsCollector.to_string()),
                    denom: initial_market.denom.clone(),
                    user_amount_scaled_before: Uint128::zero(),
                    total_amount_scaled_before: initial_market.collateral_total_scaled,
                })
                .unwrap(),
                funds: vec![]
            }),
            SubMsg::new(WasmMsg::Execute {
                contract_addr: MarsAddressType::Incentives.to_string(),
                msg: to_binary(&incentives::ExecuteMsg::BalanceChange {
                    user_addr: on_behalf_of_addr.clone(),
                    denom: initial_market.denom.clone(),
                    user_amount_scaled_before: Uint128::zero(),
                    // NOTE: New collateral shares were minted to the rewards collector first, so
                    // for the depositor this should be initial total supply + rewards shares minted
                    total_amount_scaled_before: initial_market.collateral_total_scaled
                        + expected_reward_amount_scaled,
                })
                .unwrap(),
                funds: vec![]
            })
        ]
    );

    // depositor should not have created a new collateral position
    let opt = COLLATERALS.may_load(deps.as_ref().storage, (&depositor_addr, denom)).unwrap();
    assert!(opt.is_none());

    // the recipient should have created a new collateral position
    let collateral = COLLATERALS.load(deps.as_ref().storage, (&on_behalf_of_addr, denom)).unwrap();
    assert_eq!(
        collateral,
        Collateral {
            amount_scaled: expected_mint_amount,
            enabled: true,
        }
    );
}

#[test]
fn depositing_on_behalf_of_cannot_enable_collateral() {
    let TestSuite {
        mut deps,
        denom,
        depositor_addr,
        ..
    } = setup_test();

    deps.querier.set_oracle_price(denom, Decimal::one());

    let on_behalf_of_addr = Addr::unchecked("jake");

    let block_time = 10000300;

    // 'on_behalf_of_addr' deposit funds to their own account
    execute(
        deps.as_mut(),
        mock_env_at_block_time(block_time),
        mock_info(on_behalf_of_addr.as_str(), &coins(1u128, denom)),
        ExecuteMsg::Deposit {
            on_behalf_of: None,
        },
    )
    .unwrap();

    // 'on_behalf_of_addr' should have collateral enabled
    let collateral = COLLATERALS.load(deps.as_ref().storage, (&on_behalf_of_addr, denom)).unwrap();
    assert!(collateral.enabled);

    // 'on_behalf_of_addr' disables asset as collateral
    execute(
        deps.as_mut(),
        mock_env_at_block_time(block_time),
        mock_info(on_behalf_of_addr.as_str(), &[]),
        ExecuteMsg::UpdateAssetCollateralStatus {
            denom: denom.to_string(),
            enable: false,
        },
    )
    .unwrap();

    // verify asset is disabled as collateral for 'on_behalf_of_addr'
    let collateral = COLLATERALS.load(deps.as_ref().storage, (&on_behalf_of_addr, denom)).unwrap();
    assert!(!collateral.enabled);

    // 'depositor_addr' deposits a small amount of funds to 'on_behalf_of_addr' to enable his asset as collateral
    execute(
        deps.as_mut(),
        mock_env_at_block_time(block_time),
        mock_info(depositor_addr.as_str(), &coins(1u128, denom)),
        ExecuteMsg::Deposit {
            on_behalf_of: Some(on_behalf_of_addr.to_string()),
        },
    )
    .unwrap();

    // 'on_behalf_of_addr' doesn't have the asset enabled as collateral
    let collateral = COLLATERALS.load(deps.as_ref().storage, (&on_behalf_of_addr, denom)).unwrap();
    assert!(!collateral.enabled);
}
