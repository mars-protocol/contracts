use cosmwasm_std::{coin, Addr};
use mars_v3_zapper_base::msg::CallbackMsg;
use osmosis_test_tube::Account;

use crate::helpers::{assert_err, MockEnv};

pub mod helpers;

#[test]
fn only_owner_can_invoke_callback() {
    let mut mock = MockEnv::new().build().unwrap();
    let bad_guy = mock.app.init_account(&[coin(1_000_000, "uosmo")]).unwrap();
    let err = mock
        .callback(
            CallbackMsg::RefundCoin {
                recipient: Addr::unchecked(bad_guy.address()),
                denoms: vec!["xyz".to_string()],
            },
            Some(&bad_guy),
        )
        .unwrap_err();

    assert_err(err, "Caller not permitted to perform action");
}
