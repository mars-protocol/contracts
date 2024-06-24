use cosmwasm_std::{coin, coins, Addr, Decimal, Uint128};
use mars_credit_manager::error::{ContractError, ContractError::NotLiquidatable};
use mars_mock_oracle::msg::CoinPrice;
use mars_testing::multitest::helpers::coin_info;
use mars_types::{
    credit_manager::{
        Action::{Borrow, Deposit, Liquidate, StakeAstroLp},
        LiquidateRequest,
    },
    health::AccountKind,
    oracle::ActionKind,
};

use super::helpers::{
    assert_err, get_coin, get_debt, uatom_info, ujake_info, uosmo_info, AccountToFund, MockEnv,
};

#[test]
fn staked_lp_positions_contribute_to_health() {
    let uatom_info = uatom_info();
    let uosmo_info = uosmo_info();
    let lp_info = coin_info("factory12345");

    let liquidatee = Addr::unchecked("liquidatee");
    let mut mock = MockEnv::new()
        .set_params(&[uatom_info.clone(), uosmo_info.clone(), lp_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![uatom_info.to_coin(500), uosmo_info.to_coin(500), lp_info.to_coin(200)],
        })
        .build()
        .unwrap();

    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uatom_info.to_coin(100)),
            Deposit(lp_info.to_coin(50)),
            Borrow(uosmo_info.to_coin(40)),
        ],
        &[uatom_info.to_coin(100), lp_info.to_coin(50)],
    )
    .unwrap();

    let health_1 =
        mock.query_health(&liquidatee_account_id, AccountKind::Default, ActionKind::Liquidation);
    assert!(!health_1.liquidatable);
    // 100 uatom * 1 + 50 lp * 0.25 + 40 uosmo * 0.25 = 100 + 12.5 + 10 = 122.5 ~ 122
    assert_eq!(health_1.total_collateral_value, Uint128::new(122u128));

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![StakeAstroLp {
            lp_token: lp_info.to_action_coin(50),
        }],
        &[],
    )
    .unwrap();

    // Collateral should be the same after staking
    let health_2 =
        mock.query_health(&liquidatee_account_id, AccountKind::Default, ActionKind::Liquidation);
    assert!(!health_2.liquidatable);
    assert_eq!(health_1.total_collateral_value, health_2.total_collateral_value);
    assert_eq!(health_1.max_ltv_adjusted_collateral, health_2.max_ltv_adjusted_collateral);
    assert_eq!(
        health_1.liquidation_threshold_adjusted_collateral,
        health_2.liquidation_threshold_adjusted_collateral
    );

    let liquidator = Addr::unchecked("liquidator");
    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![Liquidate {
            liquidatee_account_id: liquidatee_account_id.clone(),
            debt_coin: uosmo_info.to_coin(10),
            request: LiquidateRequest::StakedAstroLp(lp_info.denom),
        }],
        &[],
    );

    assert_err(
        res,
        NotLiquidatable {
            account_id: liquidatee_account_id,
            lqdt_health_factor: "9.636363636363636363".to_string(),
        },
    )
}

#[test]
fn liquidatee_does_not_have_requested_staked_lp_coin() {
    let lp_info = coin_info("factory12345");
    let uosmo_info = uosmo_info();
    let ujake_info = ujake_info();

    let liquidatee = Addr::unchecked("liquidatee");
    let liquidator = Addr::unchecked("liquidator");

    let mut mock = MockEnv::new()
        .set_params(&[lp_info.clone(), uosmo_info.clone(), ujake_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: vec![lp_info.to_coin(500)],
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: vec![uosmo_info.to_coin(500)],
        })
        .build()
        .unwrap();

    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(lp_info.to_coin(100)),
            StakeAstroLp {
                lp_token: lp_info.to_action_coin(100),
            },
            Borrow(uosmo_info.to_coin(100)),
        ],
        &[lp_info.to_coin(100)],
    )
    .unwrap();

    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uosmo_info.denom.clone(),
        price: Decimal::from_atomics(20u128, 0).unwrap(),
    });

    let health =
        mock.query_health(&liquidatee_account_id, AccountKind::Default, ActionKind::Liquidation);
    assert!(health.liquidatable);

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    let res = mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uosmo_info.to_coin(10)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uosmo_info.to_coin(10),
                request: LiquidateRequest::StakedAstroLp(ujake_info.denom),
            },
        ],
        &[uosmo_info.to_coin(10)],
    );

    assert_err(res, ContractError::NoAstroLp);
}

/// Liquidation numbers based on `lent_position_partially_liquidated` in spreadsheed. Only difference is that
/// the liquidator is liquidating the staked LP token instead of the lend asset.
#[test]
fn staked_lp_position_partially_liquidated() {
    // uosmo is used as LP token
    let uosmo_info = uosmo_info();
    let uatom_info = uatom_info();

    let liquidator = Addr::unchecked("liquidator");
    let liquidatee = Addr::unchecked("liquidatee");

    let mut mock = MockEnv::new()
        .target_health_factor(Decimal::from_atomics(12u128, 1).unwrap())
        .set_params(&[uosmo_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: liquidatee.clone(),
            funds: coins(2000, uosmo_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: liquidator.clone(),
            funds: coins(2000, uatom_info.denom.clone()),
        })
        .build()
        .unwrap();

    let liquidatee_account_id = mock.create_credit_account(&liquidatee).unwrap();

    mock.update_credit_account(
        &liquidatee_account_id,
        &liquidatee,
        vec![
            Deposit(uosmo_info.to_coin(1050)),
            Borrow(uatom_info.to_coin(1000)),
            StakeAstroLp {
                lp_token: uosmo_info.to_action_coin(450),
            },
        ],
        &[uosmo_info.to_coin(1050)],
    )
    .unwrap();

    // Add rewards
    let astro_reward = coin(54, "uastro");
    let atom_reward = coin(4, "uatom");
    mock.add_astro_incentive_reward(
        &liquidatee_account_id,
        &uosmo_info.denom,
        astro_reward.clone(),
    );
    mock.add_astro_incentive_reward(&liquidatee_account_id, &uosmo_info.denom, atom_reward.clone());

    mock.price_change(CoinPrice {
        pricing: ActionKind::Liquidation,
        denom: uatom_info.denom.clone(),
        price: Decimal::from_atomics(22u128, 1).unwrap(),
    });

    let health =
        mock.query_health(&liquidatee_account_id, AccountKind::Default, ActionKind::Liquidation);
    assert!(health.liquidatable);
    assert_eq!(health.total_collateral_value, Uint128::new(2462u128));
    assert_eq!(health.total_debt_value, Uint128::new(2203u128));

    let liquidator_account_id = mock.create_credit_account(&liquidator).unwrap();

    mock.update_credit_account(
        &liquidator_account_id,
        &liquidator,
        vec![
            Deposit(uatom_info.to_coin(45)),
            Liquidate {
                liquidatee_account_id: liquidatee_account_id.clone(),
                debt_coin: uatom_info.to_coin(45),
                request: LiquidateRequest::StakedAstroLp(uosmo_info.denom),
            },
        ],
        &[uatom_info.to_coin(45)],
    )
    .unwrap();

    // Assert liquidatee's new position
    let position = mock.query_positions(&liquidatee_account_id);
    assert_eq!(position.deposits.len(), 3);
    let osmo_balance = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_balance.amount, Uint128::new(600));
    let atom_balance = get_coin("uatom", &position.deposits);
    assert_eq!(atom_balance.amount, Uint128::new(1000) + atom_reward.amount);
    let astro_balance = get_coin(&astro_reward.denom, &position.deposits);
    assert_eq!(astro_balance.amount, astro_reward.amount);

    assert_eq!(position.debts.len(), 1);
    let atom_debt = get_debt("uatom", &position.debts);
    assert_eq!(atom_debt.amount, Uint128::new(956));

    assert_eq!(position.staked_astro_lps.len(), 1);
    let osmo_staked_lp = get_coin("uosmo", &position.staked_astro_lps);
    assert_eq!(osmo_staked_lp.amount, Uint128::new(46));

    assert_eq!(position.lends.len(), 0);

    // Assert liquidator's new position
    let position = mock.query_positions(&liquidator_account_id);
    assert_eq!(position.staked_astro_lps.len(), 0);
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.debts.len(), 0);
    assert_eq!(position.deposits.len(), 1);
    let osmo_deposited = get_coin("uosmo", &position.deposits);
    assert_eq!(osmo_deposited.amount, Uint128::new(400));

    // Assert rewards-collector's new position
    let rewards_collector_acc_id = mock.query_rewards_collector_account();
    let position = mock.query_positions(&rewards_collector_acc_id);
    assert_eq!(position.deposits.len(), 1);
    let rc_osmo_deposited = get_coin("uosmo", &position.deposits);
    assert_eq!(rc_osmo_deposited.amount, Uint128::new(4));
    assert_eq!(position.lends.len(), 0);
    assert_eq!(position.staked_astro_lps.len(), 0);
    assert_eq!(position.debts.len(), 0);

    // Liq HF should improve
    let account_kind = mock.query_account_kind(&liquidatee_account_id);
    let health = mock.query_health(&liquidatee_account_id, account_kind, ActionKind::Liquidation);
    assert!(!health.liquidatable);
}
