use crate::helpers::default_asset_params;
use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_testing::integration::mock_env::MockEnvBuilder;

mod helpers;

#[test]
fn userflow() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    // setup oracle and red-bank
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_fixed(&mut mock_env, "uatom", Decimal::from_ratio(12u128, 1u128));
    oracle.set_price_source_fixed(&mut mock_env, "uosmo", Decimal::from_ratio(15u128, 10u128));
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::one());
    let red_bank = mock_env.red_bank.clone();
    red_bank.init_asset(&mut mock_env, "uatom", default_asset_params());
    red_bank.init_asset(&mut mock_env, "uosmo", default_asset_params());
    red_bank.init_asset(&mut mock_env, "uusdc", default_asset_params());

    // fund user account
    let user = Addr::unchecked("user");
    let funded_atom = 100_000_000u128;
    mock_env.fund_account(&user, &[coin(funded_atom, "uatom")]);
    let balance = mock_env.query_balance(&user, "uatom").unwrap();
    assert_eq!(balance.amount.u128(), 100_000_000u128);

    // check if red-bank doesn't have collateral for the user
    let collateral = red_bank.query_user_collateral(&mut mock_env, &user, "uatom");
    assert_eq!(collateral.amount, Uint128::zero());

    // move few blocks
    mock_env.increment_by_blocks(10);

    // deposit some atom and check if balance is correct
    let deposited_atom = 65_000_000u128;
    red_bank.deposit(&mut mock_env, &user, coin(deposited_atom, "uatom")).unwrap();
    let balance = mock_env.query_balance(&user, "uatom").unwrap();
    assert_eq!(balance.amount.u128(), funded_atom - deposited_atom);
    let collateral = red_bank.query_user_collateral(&mut mock_env, &user, "uatom");
    assert_eq!(collateral.amount.u128(), deposited_atom);
}
