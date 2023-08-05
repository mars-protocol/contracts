// must be public module so that clippy doesn't complain "dead code"
pub mod helpers;

use std::collections::HashMap;

use cosmwasm_std::{Addr, Coin, Coins, Decimal, StdResult, Uint128};
use mars_params::{msg::AssetParamsUpdate, types::asset::AssetParams};
use mars_rover::{
    error::ContractError,
    msg::execute::{Action, ActionAmount, ActionCoin},
};
use test_case::test_case;

use crate::helpers::{uatom_info, uosmo_info, AccountToFund, MockEnv};

#[test_case(
    [].into(),
    vec![
        Action::Deposit(Coin {
            denom: "uatom".into(),
            amount: Uint128::new(123),
        }),
        Action::Deposit(Coin {
            denom: "uosmo".into(),
            amount: Uint128::new(456),
        }),
    ],
    true;
    "no deposit cap"
)]
#[test_case(
    [("uatom", 100)].into(),
    vec![
        Action::Deposit(Coin {
            denom: "uatom".into(),
            amount: Uint128::new(101), // this exceeds the cap of 100
        }),
        Action::Deposit(Coin {
            denom: "uosmo".into(),
            amount: Uint128::new(456),
        }),
    ],
    false;
    "deposit cap exceeded"
)]
#[test_case(
    [("uatom", 100)].into(),
    vec![
        // this first action exceeds deposit cap...
        Action::Deposit(Coin {
            denom: "uatom".into(),
            amount: Uint128::new(101),
        }),
        // but we immediately does a swap to uatom, which does not exceed cap
        // therefore, the tx should be successful
        Action::SwapExactIn {
            coin_in: ActionCoin {
                denom: "uatom".into(),
                amount: ActionAmount::AccountBalance,
            },
            denom_out: "uosmo".into(),
            slippage: Decimal::one(),
        }
    ],
    true;
    "a deposit action causes cap to be exceeded but a follow up swap action saves it"
)]
#[test_case(
    // in our specific test setup, 123 uatom swaps to 1337 uosmo
    // we set the cap to 1000 uosmo which should be exceeded
    [("uatom", 200), ("uosmo", 1000)].into(),
    vec![
        Action::Deposit(Coin {
            denom: "uatom".into(),
            amount: Uint128::new(123),
        }),
        Action::SwapExactIn {
            coin_in: ActionCoin {
                denom: "uatom".into(),
                amount: ActionAmount::AccountBalance,
            },
            denom_out: "uosmo".into(),
            slippage: Decimal::one(),
        }
    ],
    false;
    "a deposit action is below cap but a follow up swap action exceeds the cap"
)]
fn asserting_deposit_cap(
    deposit_caps: HashMap<&'static str, u128>,
    actions: Vec<Action>,
    exp_ok: bool,
) {
    let user = Addr::unchecked("user");

    // compute how much coins need to be sent to the contract in order to update
    // the credit account
    let send_funds = actions
        .iter()
        .try_fold(Coins::default(), |mut coins, action| -> StdResult<_> {
            if let Action::Deposit(coin) = action {
                coins.add(coin.clone())?;
            }
            Ok(coins)
        })
        .unwrap()
        .to_vec();

    // set up mock environment
    let mut mock = MockEnv::new()
        .set_params(&[uosmo_info(), uatom_info()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: send_funds.clone(),
        })
        .build()
        .unwrap();

    // set deposit caps for uosmo and uatom
    // the `uosmo_info` and `uatom_info` functions set the cap to Uint128::MAX,
    // so here we need to update them to our intended value for the purpose of
    // this test
    for (denom, cap) in deposit_caps {
        let mut params: AssetParams = mock.query_asset_params(denom);
        params.deposit_cap = cap.into();
        mock.update_asset_params(AssetParamsUpdate::AddOrUpdate {
            params: params.into(),
        });
    }

    // register an account
    let account_id = mock.create_credit_account(&user).unwrap();

    // attempt to execute the actions
    let result = mock.update_credit_account(&account_id, &user, actions, &send_funds);

    if exp_ok {
        assert!(result.is_ok());
    } else {
        let err: ContractError = result.unwrap_err().downcast().unwrap();
        // if errors, we make sure the error is the AboveAssetDepositCap error
        // and not any other error
        assert!(matches!(err, ContractError::AboveAssetDepositCap { .. }));
    }
}
