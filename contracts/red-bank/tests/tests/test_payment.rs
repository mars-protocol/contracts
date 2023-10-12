use cosmwasm_std::{
    coins,
    testing::{mock_env, mock_info},
    Uint128,
};
use cw_utils::PaymentError;
use mars_red_bank::contract;
use mars_types::red_bank::ExecuteMsg;

use super::helpers::th_setup;

/// The Red Bank contract has 6 user-facing functions: deposit, withdraw, borrow,
/// repay, liquidate, and update_asset_collateral_status; amount these, 3 do not
/// expect the user to send any payment. This test verifies that they properly
/// reject if a user sends an expected payment.
///
/// This is in response to this mainnet tx, where a user sends a payment with a
/// `withdraw` msg:
/// https://www.mintscan.io/osmosis/txs/2F214EE3A22DC93E61DE9A49BE616B317EB28AFC5E43B0AF07800AC7E6435522
#[test]
fn rejecting_unexpected_payments() {
    let mut deps = th_setup(&[]);

    let err = contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info("larry", &coins(123, "uosmo")),
        ExecuteMsg::Withdraw {
            denom: "".into(),
            amount: None,
            recipient: None,
            account_id: None,
            liquidation_related: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, PaymentError::NonPayable {}.into());

    let err = contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info("larry", &coins(234, "umars")),
        ExecuteMsg::Borrow {
            denom: "".into(),
            amount: Uint128::zero(),
            recipient: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, PaymentError::NonPayable {}.into());

    let err = contract::execute(
        deps.as_mut(),
        mock_env(),
        mock_info("larry", &coins(345, "uluna")),
        ExecuteMsg::UpdateAssetCollateralStatus {
            denom: "".into(),
            enable: false,
        },
    )
    .unwrap_err();
    assert_eq!(err, PaymentError::NonPayable {}.into());
}
