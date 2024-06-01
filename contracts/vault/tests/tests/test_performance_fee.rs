use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Decimal, Int128, Uint128};
use cw_multi_test::{BankSudo, SudoMsg};
use mars_mock_oracle::msg::CoinPrice;
use mars_testing::multitest::helpers::{
    coin_info, deploy_managed_vault_with_performance_fee, uatom_info, CoinInfo,
};
use mars_types::{credit_manager::Action, health::AccountKind, oracle::ActionKind};
use mars_vault::{error::ContractError, msg::PerformanceFeeConfig, state::PerformanceFeeState};

use super::{
    helpers::{AccountToFund, MockEnv},
    vault_helpers::{assert_vault_err, execute_withdraw_performance_fee},
};
use crate::tests::{
    helpers::deploy_managed_vault,
    vault_helpers::{
        execute_deposit, execute_redeem, execute_unlock, query_performance_fee, query_vault_info,
    },
};

#[test]
fn deposit_if_credit_manager_account_not_binded() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(1_000_000_000, "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);

    let res = execute_withdraw_performance_fee(&mut mock, &user, &managed_vault_addr, None);
    assert_vault_err(res, ContractError::VaultAccountNotFound {});
}

#[test]
fn unauthorized_performance_fee_withdraw() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let user_funded_amt = Uint128::new(1_000_000_000);
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(user_funded_amt.u128(), "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);

    let vault_acc_id = mock
        .create_credit_account_v2(
            &fund_manager,
            AccountKind::FundManager {
                vault_addr: managed_vault_addr.to_string(),
            },
            None,
        )
        .unwrap();

    // vault user can't withdraw performance fee
    let res = execute_withdraw_performance_fee(&mut mock, &user, &managed_vault_addr, None);
    assert_vault_err(
        res,
        ContractError::NotTokenOwner {
            user: user.to_string(),
            account_id: vault_acc_id.clone(),
        },
    );

    // random user can't withdraw performance fee
    let random_user = Addr::unchecked("random-user");
    let res = execute_withdraw_performance_fee(&mut mock, &random_user, &managed_vault_addr, None);
    assert_vault_err(
        res,
        ContractError::NotTokenOwner {
            user: random_user.to_string(),
            account_id: vault_acc_id,
        },
    );
}

/// Scenarios based on spreadsheet:
/// ../files/Mars - 3rd party Vault - Performance Fee - test cases v1.0.xlsx
#[test]
fn performance_fee_correctly_accumulated() {
    let uusdc_info = coin_info("uusdc");
    let uatom_info = uatom_info();

    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let user_funded_amt = Uint128::new(100_000_000_000);
    let mut mock = MockEnv::new()
        .set_params(&[uusdc_info.clone(), uatom_info.clone()])
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(user_funded_amt.u128(), "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault_with_performance_fee(
        &mut mock.app,
        &fund_manager,
        &credit_manager,
        0,
        PerformanceFeeConfig {
            performance_fee_percentage: Decimal::from_str("0.0000208").unwrap(),
            performance_fee_interval: 60,
        },
    );

    let fund_acc_id = mock
        .create_credit_account_v2(
            &fund_manager,
            AccountKind::FundManager {
                vault_addr: managed_vault_addr.to_string(),
            },
            None,
        )
        .unwrap();

    // simulate base token price = 1 USD
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uusdc_info.denom.clone(),
        price: Decimal::one(),
    });

    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let vault_token = vault_info_res.vault_token;

    // there shouldn't be any base tokens in Fund Manager wallet
    let base_token_balance = mock.query_balance(&fund_manager, &uusdc_info.denom.clone()).amount;
    assert!(base_token_balance.is_zero());

    // -- FIRST ACTION --

    let first_deposit_time = mock.query_block_time();
    let deposited_amt = Uint128::new(100_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            updated_at: first_deposit_time,
            liquidity: deposited_amt,
            accumulated_pnl: Int128::zero(),
            accumulated_fee: Uint128::zero()
        }
    );

    // swap USDC to ATOM to tune PnL value based on different ATOM price
    swap_usdc_to_atom(&mut mock, &fund_acc_id, &fund_manager, &uusdc_info, &uatom_info);

    // -- SECOND ACTION --

    // move by 97 hours
    mock.increment_by_time(97 * 60 * 60);

    let pnl = calculate_pnl(&mut mock, &fund_acc_id, Decimal::from_str("1.25").unwrap());
    assert_eq!(pnl, Uint128::new(120_000_000));

    let deposited_amt = Uint128::new(20_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            updated_at: first_deposit_time,
            liquidity: Uint128::new(139959648),
            accumulated_pnl: Int128::new(20000000),
            accumulated_fee: Uint128::new(40352)
        }
    );

    // -- THIRD ACTION --

    // move by 72 hours
    mock.increment_by_time(72 * 60 * 60);

    let pnl = calculate_pnl(&mut mock, &fund_acc_id, Decimal::from_str("0.25").unwrap());
    assert_eq!(pnl, Uint128::new(60_000_000));

    let deposited_amt = Uint128::new(15_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            updated_at: first_deposit_time,
            liquidity: Uint128::new(75000000),
            accumulated_pnl: Int128::new(-59959648),
            accumulated_fee: Uint128::zero()
        }
    );

    // -- FOURTH ACTION --

    // move by 144 hours
    mock.increment_by_time(144 * 60 * 60);

    // we have 55_000_000 uusdc + 80_000_000 uatom
    // we want to have pnl = 450_000_000 uusdc so uatom has to be worth 450_000_000 - 55_000_000 = 395_000_000
    // so the price of uatom has to be 395_000_000 / 80_000_000 = 4.9375
    let pnl = calculate_pnl(&mut mock, &fund_acc_id, Decimal::from_str("4.9375").unwrap());
    assert_eq!(pnl, Uint128::new(450_000_000));

    let unlock_vault_tokens = Uint128::new(10_000_000_000_000);
    execute_unlock(&mut mock, &user, &managed_vault_addr, unlock_vault_tokens, &[]).unwrap();
    execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(unlock_vault_tokens.u128(), vault_token.clone())],
    )
    .unwrap();

    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            updated_at: first_deposit_time,
            liquidity: Uint128::new(417233938),
            accumulated_pnl: Int128::new(315040352),
            accumulated_fee: Uint128::new(2051038)
        }
    );

    // -- FIFTH ACTION --

    // move by 744 hours
    mock.increment_by_time(744 * 60 * 60);

    let pnl = calculate_pnl(&mut mock, &fund_acc_id, Decimal::from_str("10").unwrap());
    assert_eq!(pnl, Uint128::new(824284976));

    execute_withdraw_performance_fee(
        &mut mock,
        &fund_manager,
        &managed_vault_addr,
        Some(PerformanceFeeConfig {
            performance_fee_percentage: Decimal::from_str("0.0000408").unwrap(),
            performance_fee_interval: 60,
        }),
    )
    .unwrap();

    let fee_withdraw_time = mock.query_block_time();
    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            updated_at: fee_withdraw_time,
            liquidity: Uint128::new(808409364),
            accumulated_pnl: Int128::zero(),
            accumulated_fee: Uint128::zero()
        }
    );

    let base_token_balance = mock.query_balance(&fund_manager, &uusdc_info.denom.clone()).amount;
    assert_eq!(base_token_balance, Uint128::new(15875612));

    // -- SIXTH ACTION --

    // move by 48 hours
    mock.increment_by_time(48 * 60 * 60);

    let pnl = calculate_pnl(&mut mock, &fund_acc_id, Decimal::from_str("10.5").unwrap());
    assert_eq!(pnl, Uint128::new(848409364));

    let deposited_amt = Uint128::new(55_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    // new performance fee percentage should be used
    let performance_fee = query_performance_fee(&mock, &managed_vault_addr);
    assert_eq!(
        performance_fee,
        PerformanceFeeState {
            updated_at: fee_withdraw_time,
            liquidity: Uint128::new(903331028),
            accumulated_pnl: Int128::new(40000000),
            accumulated_fee: Uint128::new(78336)
        }
    );
}

fn swap_usdc_to_atom(
    mock: &mut MockEnv,
    fund_acc_id: &str,
    fund_manager: &Addr,
    uusdc_info: &CoinInfo,
    uatom_info: &CoinInfo,
) {
    let swap_amt = Uint128::new(80_000_000);
    let cm_config = mock.query_config();
    mock.app
        .sudo(SudoMsg::Bank(BankSudo::Mint {
            to_address: cm_config.swapper,
            amount: vec![coin(swap_amt.u128(), uatom_info.denom.clone())],
        }))
        .unwrap();
    mock.update_credit_account(
        fund_acc_id,
        fund_manager,
        vec![Action::SwapExactIn {
            coin_in: uusdc_info.to_action_coin(swap_amt.u128()),
            denom_out: uatom_info.denom.clone(),
            slippage: Decimal::from_atomics(6u128, 1).unwrap(),
            route: None,
        }],
        &[],
    )
    .unwrap();
}

fn calculate_pnl(mock: &mut MockEnv, fund_acc_id: &str, new_atom_price: Decimal) -> Uint128 {
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: "uatom".to_string(),
        price: new_atom_price,
    });

    let res = mock.query_positions(fund_acc_id);
    assert_eq!(res.deposits.len(), 2);

    let mut pnl = Uint128::zero();
    for deposit in res.deposits.iter() {
        let price = mock.query_price(&deposit.denom).price;
        let value = deposit.amount * price;
        pnl += value;
    }

    pnl
}
