use std::str::FromStr;

use cosmwasm_std::{coin, Addr, Decimal, Uint128};

use super::helpers::{assert_err, AccountToFund, MockEnv};

#[test]
fn deposit_to_vault() {
    let user = Addr::unchecked("user");

    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![coin(300, "uusdc"), coin(400, "coin_2")],
        })
        .build()
        .unwrap();

    // mock.deposit(&user, Uint128::new(100), &[coin(100, "uusdc")]).unwrap();
}
