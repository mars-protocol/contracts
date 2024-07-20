use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use cw_utils::PaymentError;
use mars_mock_oracle::msg::CoinPrice;
use mars_testing::multitest::helpers::{uosmo_info, CoinInfo};
use mars_types::{
    credit_manager::{Action, ActionAmount, ActionCoin},
    health::AccountKind,
    oracle::ActionKind,
    params::LiquidationBonus,
};
use mars_vault::error::ContractError;
use test_case::test_case;

use super::{
    helpers::{AccountToFund, MockEnv},
    vault_helpers::{assert_vault_err, execute_deposit, execute_redeem, execute_unlock},
};
use crate::tests::{
    helpers::deploy_managed_vault,
    vault_helpers::{query_convert_to_assets, query_convert_to_shares, query_vault_info},
};

#[test]
fn redeem_invalid_funds() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(1_000_000_000, "untrn"), coin(1_000_000_000, "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);

    mock.create_credit_account_v2(
        &fund_manager,
        AccountKind::FundManager {
            vault_addr: managed_vault_addr.to_string(),
        },
        None,
    )
    .unwrap();

    let res = execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[],
    );
    assert_vault_err(res, ContractError::Payment(PaymentError::NoFunds {}));

    let res = execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(1_001, "untrn"), coin(1_002, "uusdc")],
    );
    assert_vault_err(res, ContractError::Payment(PaymentError::MultipleDenoms {}));

    let res = execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(1_001, "untrn")],
    );
    assert_vault_err(
        res,
        ContractError::Payment(PaymentError::MissingDenom("factory/contract11/vault".to_string())),
    );
}

#[test]
fn redeem_if_credit_manager_account_not_binded() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(1_000_000_000, "vault")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);

    let deposited_amt = Uint128::new(123_000_000);
    let res = execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "vault")],
    );
    assert_vault_err(res, ContractError::VaultAccountNotFound {});
}

#[test]
fn redeem_if_unlocked_positions_not_found() {
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
    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let vault_token = vault_info_res.vault_token;

    mock.create_credit_account_v2(
        &fund_manager,
        AccountKind::FundManager {
            vault_addr: managed_vault_addr.to_string(),
        },
        None,
    )
    .unwrap();

    // deposit to get vault tokens
    let deposited_amt = Uint128::new(123_000_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    let res = execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(10u128, vault_token.clone())],
    );
    assert_vault_err(res, ContractError::UnlockedPositionsNotFound {});
}

#[test]
fn redeem_invalid_unlocked_amount() {
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
    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let vault_token = vault_info_res.vault_token;

    mock.create_credit_account_v2(
        &fund_manager,
        AccountKind::FundManager {
            vault_addr: managed_vault_addr.to_string(),
        },
        None,
    )
    .unwrap();

    let deposited_amt = Uint128::new(12_400_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    let user_vault_token_balance = mock.query_balance(&user, &vault_token).amount;
    let first_unlock = user_vault_token_balance.multiply_ratio(1u128, 4u128);
    let second_unlock = user_vault_token_balance.multiply_ratio(1u128, 4u128);

    execute_unlock(&mut mock, &user, &managed_vault_addr, first_unlock, &[]).unwrap();

    // move time forward to create new unlock entry
    mock.increment_by_time(5);

    execute_unlock(&mut mock, &user, &managed_vault_addr, second_unlock, &[]).unwrap();

    // try to redeem when cooldown period hasn't passed yet
    let res = execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(10u128, vault_token.clone())],
    );
    assert_vault_err(res, mars_vault::error::ContractError::UnlockedPositionsNotFound {});

    // move time forward to pass cooldown period
    mock.increment_by_time(vault_info_res.cooldown_period + 1);

    let vault_tokens = first_unlock + second_unlock - Uint128::one();
    let res = execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(vault_tokens.u128(), vault_token.clone())],
    );
    assert_vault_err(
        res,
        ContractError::InvalidAmount {
            reason: "provided vault tokens is less than total unlocked amount".to_string(),
        },
    );
}

#[test]
fn redeem_with_refund() {
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let user_funded_amt = Uint128::new(1_000_000_000);
    let mut mock = MockEnv::new()
        .set_params(&[uusdc_info()])
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
    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let vault_token = vault_info_res.vault_token;

    mock.create_credit_account_v2(
        &fund_manager,
        AccountKind::FundManager {
            vault_addr: managed_vault_addr.to_string(),
        },
        None,
    )
    .unwrap();

    let deposited_amt = Uint128::new(12_400_000);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(deposited_amt.u128(), "uusdc")],
    )
    .unwrap();

    let user_vault_token_balance_before = mock.query_balance(&user, &vault_token).amount;
    let unlock = user_vault_token_balance_before.multiply_ratio(1u128, 4u128);

    execute_unlock(&mut mock, &user, &managed_vault_addr, unlock, &[]).unwrap();

    // move time forward to pass cooldown period
    mock.increment_by_time(vault_info_res.cooldown_period + 1);

    let refund_amt = Uint128::new(123);

    let vault_tokens = unlock + refund_amt;
    execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(vault_tokens.u128(), vault_token.clone())],
    )
    .unwrap();

    let contract_vault_token_balance = mock.query_balance(&managed_vault_addr, &vault_token).amount;
    assert!(contract_vault_token_balance.is_zero());

    // vault tokens should be refunded
    let user_vault_token_balance = mock.query_balance(&user, &vault_token).amount;
    assert_eq!(user_vault_token_balance, user_vault_token_balance_before - unlock);
}

/// There are rounding errors when converting back and forth between base tokens and vault tokens so there could be a difference of 1 base token.
/// Also, there could be yield simulated for lend and debt - +1 to lend and -1 to debt.
#[test_case(2_000_000_000, 0, 2_000_000_000, 1, 0, 0; "redeem from deposit if no lend, dust left")]
#[test_case(2_000_000_000, 0, 2_000_000_001, 0, 0, 0; "redeem from deposit if no lend")]
#[test_case(2_000_000_000, 1_000_000_000, 500_000_000, 1_500_000_001, 1_000_000_001, 0; "redeem from deposit if lend available")]
#[test_case(2_000_000_000, 1_000_000_000, 2_200_000_000, 0, 800_000_002, 0; "redeem from deposit and lend")]
#[test_case(2_000_000_000, 1_000_000_000, 3_200_000_000, 0, 0, 199_999_999; "redeem from deposit, lend and debt")]
#[test_case(5_000_000_000, 2_000_000_000, 7_800_000_000, 0, 0, 0 => panics "Actions resulted in exceeding maximum allowed loan-to-value."; "redeem more than HF limit")]
fn redeem_succeded(
    deposit_amt: u128,
    lend_amt: u128,
    requested_base_tokens: u128,
    expected_deposit_amt: u128,
    expected_lend_amt: u128,
    expected_debt_amt: u128,
) {
    let swap_amt = deposit_amt;

    let uusdc_info = uusdc_info();
    let uosmo_info = uosmo_info();

    let liquidity_provider = Addr::unchecked("liquidity-provider");
    let fund_manager = Addr::unchecked("fund-manager");
    let user = Addr::unchecked("user");
    let liquidity_provided_amt = Uint128::new(1_000_000_000_000);
    let user_funded_amt = Uint128::new(100_000_000_000);
    let mut mock = MockEnv::new()
        .set_params(&[uusdc_info.clone(), uosmo_info.clone()])
        .fund_account(AccountToFund {
            addr: fund_manager.clone(),
            funds: vec![coin(1_000_000_000, "untrn")],
        })
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(user_funded_amt.u128(), "uusdc")],
        })
        .fund_account(AccountToFund {
            addr: liquidity_provider.clone(),
            funds: vec![coin(liquidity_provided_amt.u128(), "uusdc")],
        })
        .build()
        .unwrap();
    let credit_manager = mock.rover.clone();

    // provide liquidity into red bank
    let account_id = mock.create_credit_account(&liquidity_provider).unwrap();
    let liquidity_coin = coin(liquidity_provided_amt.u128(), "uusdc");
    mock.update_credit_account(
        &account_id,
        &liquidity_provider,
        vec![
            Action::Deposit(liquidity_coin.clone()),
            Action::Lend(ActionCoin {
                denom: "uusdc".to_string(),
                amount: ActionAmount::AccountBalance,
            }),
        ],
        &[liquidity_coin],
    )
    .unwrap();

    let managed_vault_addr = deploy_managed_vault(&mut mock.app, &fund_manager, &credit_manager);
    let vault_info_res = query_vault_info(&mock, &managed_vault_addr);
    let vault_token = vault_info_res.vault_token;

    let fund_acc_id = mock
        .create_credit_account_v2(
            &fund_manager,
            AccountKind::FundManager {
                vault_addr: managed_vault_addr.to_string(),
            },
            None,
        )
        .unwrap();

    let mut fund_acc_amt = deposit_amt;

    let mut actions = vec![];
    if lend_amt != 0 {
        actions.push(Action::Lend(uusdc_info.to_action_coin(lend_amt)));
        fund_acc_amt += lend_amt;
    }
    let estimate_res = mock.query_swap_estimate_with_optional_route(
        &uusdc_info.to_coin(swap_amt),
        &uosmo_info.denom,
        None,
    );
    let min_receive =
        estimate_res.amount * (Decimal::one() - Decimal::from_atomics(6u128, 1).unwrap());
    actions.push(Action::SwapExactIn {
        coin_in: uusdc_info.to_action_coin(swap_amt),
        denom_out: uosmo_info.denom.clone(),
        min_receive,
        route: None,
    });
    fund_acc_amt += swap_amt;

    let fund_acc_amt = Uint128::new(fund_acc_amt);
    execute_deposit(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(fund_acc_amt.u128(), "uusdc")],
    )
    .unwrap();

    // check base token balance after deposit
    let user_base_token_balance_after_deposit = mock.query_balance(&user, "uusdc").amount;

    mock.update_credit_account(&fund_acc_id, &fund_manager, actions, &[]).unwrap();
    // Half of uusdc is swapped to uosmo (amount = MOCK_SWAP_RESULT from mocked swapper).
    // Let's update the price of uosmo to be worth more than original uusdc amount.
    mock.price_change(CoinPrice {
        pricing: ActionKind::Default,
        denom: uosmo_info.denom,
        price: Decimal::from_atomics(1_000_000u128, 0).unwrap(),
    });

    // unlock vault tokens
    let user_vault_token_balance = mock.query_balance(&user, &vault_token).amount;
    let requested_base_tokens = Uint128::new(requested_base_tokens);
    let unlock_vault_tokens =
        query_convert_to_shares(&mock, &managed_vault_addr, requested_base_tokens);
    execute_unlock(&mut mock, &user, &managed_vault_addr, unlock_vault_tokens, &[]).unwrap();

    // recalculate the amount of base tokens to be redeemed
    let unlock_base_tokens =
        query_convert_to_assets(&mock, &managed_vault_addr, unlock_vault_tokens);
    assert_eq!(unlock_base_tokens, requested_base_tokens - Uint128::one()); // rounding issue when doing back and forth conversion

    // move time forward to pass cooldown period
    mock.increment_by_time(vault_info_res.cooldown_period + 1);

    execute_redeem(
        &mut mock,
        &user,
        &managed_vault_addr,
        Uint128::zero(), // we don't care about the amount, we are using the funds
        None,
        &[coin(unlock_vault_tokens.u128(), vault_token.clone())],
    )
    .unwrap();

    // there shouldn't be any vault tokens after redeem
    let vault_token_balance = mock.query_balance(&managed_vault_addr, &vault_token).amount;
    assert!(vault_token_balance.is_zero());
    let vault_token_balance = mock.query_balance(&user, &vault_token).amount;
    assert_eq!(vault_token_balance, user_vault_token_balance - unlock_vault_tokens);

    // check base token balance after redeem
    let user_base_token_balance = mock.query_balance(&user, "uusdc").amount;
    assert_eq!(user_base_token_balance, user_base_token_balance_after_deposit + unlock_base_tokens);

    // check Fund Manager's account after redeem
    let res = mock.query_positions(&fund_acc_id);
    let pos_deposit =
        res.deposits.iter().find(|d| d.denom == "uusdc").map(|d| d.amount).unwrap_or_default();
    assert_eq!(pos_deposit.u128(), expected_deposit_amt);
    let pos_lend =
        res.lends.iter().find(|d| d.denom == "uusdc").map(|d| d.amount).unwrap_or_default();
    assert_eq!(pos_lend.u128(), expected_lend_amt);
    let pos_debt =
        res.debts.iter().find(|d| d.denom == "uusdc").map(|d| d.amount).unwrap_or_default();
    assert_eq!(pos_debt.u128(), expected_debt_amt);

    assert!(res.vaults.is_empty());
}

pub fn uusdc_info() -> CoinInfo {
    CoinInfo {
        denom: "uusdc".to_string(),
        price: Decimal::from_atomics(102u128, 2).unwrap(),
        max_ltv: Decimal::from_atomics(7u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(78u128, 2).unwrap(),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::percent(1u64),
            slope: Decimal::from_atomics(2u128, 0).unwrap(),
            min_lb: Decimal::percent(2u64),
            max_lb: Decimal::percent(10u64),
        },
        protocol_liquidation_fee: Decimal::percent(2u64),
        whitelisted: true,
        hls: None,
    }
}
