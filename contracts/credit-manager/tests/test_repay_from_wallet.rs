use cosmwasm_std::{coin, coins, Addr, Uint128};
use cw_utils::PaymentError;
use mars_rover::{
    error::ContractError,
    msg::{
        execute::Action::{Borrow, Deposit},
        instantiate::ConfigUpdates,
    },
};

use crate::helpers::{assert_err, uosmo_info, AccountToFund, MockEnv};

pub mod helpers;

#[test]
fn raises_when_sending_incorrect_funds() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(12, "abc"), coin(32, "xyz")],
        })
        .build()
        .unwrap();

    let res = mock.repay_from_wallet(&user, "123", &[]);
    assert_err(res, ContractError::Payment(PaymentError::NoFunds {}));

    let res = mock.repay_from_wallet(&user, "123", &[coin(12, "abc"), coin(32, "xyz")]);
    assert_err(res, ContractError::Payment(PaymentError::MultipleDenoms {}));
}

#[test]
fn no_debt_on_account() {
    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(12, "abc"),
        })
        .build()
        .unwrap();

    // Credit account doesn't exist
    let res = mock.repay_from_wallet(&user, "123", &[coin(12, "abc")]);
    assert_err(res, ContractError::NoDebt);

    // Exists but no debt
    let account_id = mock.create_credit_account(&user).unwrap();
    let res = mock.repay_from_wallet(&user, &account_id, &[coin(12, "abc")]);
    assert_err(res, ContractError::NoDebt);
}

#[test]
fn repay_of_less_than_total_debt() {
    let coin_info = uosmo_info();
    let debtor = Addr::unchecked("debtor");
    let repayer = Addr::unchecked("debtor");

    let repayer_starting_amount = 300;

    let mut mock = MockEnv::new()
        .allowed_coins(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: debtor.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: repayer.clone(),
            funds: coins(repayer_starting_amount, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&debtor).unwrap();

    mock.update_credit_account(
        &account_id,
        &debtor,
        vec![
            Deposit(coin(300, coin_info.denom.clone())),
            Borrow(coin(42, coin_info.denom.clone())),
        ],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    let debt_amount = mock.query_positions(&account_id).debts.first().unwrap().amount;
    assert_eq!(debt_amount, Uint128::new(43)); // simulated debt interest adds +1

    // Now that debtor is setup, we can attempt to repay from repayor
    mock.repay_from_wallet(&repayer, &account_id, &[coin(12, coin_info.denom.clone())]).unwrap();

    // Assert new debtor position
    let positions = mock.query_positions(&account_id);
    assert_eq!(1, positions.debts.len());
    assert_eq!(Uint128::new(31), positions.debts.first().unwrap().amount); // 43 - 12

    // Assert repayer wallet after repaying
    let balance = mock.query_balance(&repayer, &coin_info.denom);
    assert_eq!(Uint128::new(repayer_starting_amount - 12), balance.amount);
}

#[test]
fn repay_of_more_than_total_debt() {
    let coin_info = uosmo_info();
    let debtor = Addr::unchecked("debtor");
    let repayer = Addr::unchecked("debtor");

    let repayer_starting_amount = 300;

    let mut mock = MockEnv::new()
        .allowed_coins(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: debtor.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: repayer.clone(),
            funds: coins(repayer_starting_amount, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&debtor).unwrap();

    mock.update_credit_account(
        &account_id,
        &debtor,
        vec![
            Deposit(coin(300, coin_info.denom.clone())),
            Borrow(coin(42, coin_info.denom.clone())),
        ],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    let debt_amount = mock.query_positions(&account_id).debts.first().unwrap().amount;
    assert_eq!(debt_amount, Uint128::new(43)); // simulated debt interest adds +1

    // Note that repayer is attempting to repay 50 (more than total debt)
    mock.repay_from_wallet(&repayer, &account_id, &[coin(50, coin_info.denom.clone())]).unwrap();

    // Assert debtor has debt fully paid
    let positions = mock.query_positions(&account_id);
    assert_eq!(0, positions.debts.len());

    // Assert refund has taken place
    let balance = mock.query_balance(&repayer, &coin_info.denom);
    assert_eq!(Uint128::new(repayer_starting_amount - 43), balance.amount);
}

#[test]
fn delisted_assets_can_be_repaid() {
    let coin_info = uosmo_info();
    let debtor = Addr::unchecked("debtor");
    let repayer = Addr::unchecked("debtor");

    let mut mock = MockEnv::new()
        .allowed_coins(&[coin_info.clone()])
        .fund_account(AccountToFund {
            addr: debtor.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .fund_account(AccountToFund {
            addr: repayer.clone(),
            funds: coins(300, coin_info.denom.clone()),
        })
        .build()
        .unwrap();
    let account_id = mock.create_credit_account(&debtor).unwrap();

    mock.update_credit_account(
        &account_id,
        &debtor,
        vec![
            Deposit(coin(300, coin_info.denom.clone())),
            Borrow(coin(42, coin_info.denom.clone())),
        ],
        &[coin(300, coin_info.denom.clone())],
    )
    .unwrap();

    // Delist the asset
    let config = mock.query_config();
    mock.update_config(
        &Addr::unchecked(config.ownership.owner.unwrap()),
        ConfigUpdates {
            account_nft: None,
            allowed_coins: Some(vec![]),
            vault_configs: None,
            oracle: None,
            red_bank: None,
            max_close_factor: None,
            max_unlocking_positions: None,
            swapper: None,
            zapper: None,
            health_contract: None,
        },
    )
    .unwrap();

    let allowed_coins = mock.query_allowed_coins(None, None);
    assert_eq!(0, allowed_coins.len());

    // There should be no error in repaying for this asset
    mock.repay_from_wallet(&repayer, &account_id, &[coin(12, coin_info.denom)]).unwrap();
}
