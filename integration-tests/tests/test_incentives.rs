use cosmwasm_std::{coin, Addr, Uint128, Uint64};

use mars_testing::integration::mock_env::MockEnvBuilder;

use crate::helpers::default_asset_params;

mod helpers;

#[test]
fn test_rewards_claim() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    red_bank.init_asset(&mut mock_env, "uusdc", default_asset_params());

    let incentives = mock_env.incentives.clone();
    incentives.set_asset_incentive(&mut mock_env, "uusdc", 10);

    // fund user wallet account with usdc
    let user = Addr::unchecked("user_a");
    let funded_amt = 10_000_000_000u128;
    mock_env.fund_account(&user, &[coin(funded_amt, "uusdc")]);
    // mock_env.fund_account(&user, &[coin(funded_amt, "umars")]);

    // fund incentives contract
    mock_env.fund_account(&incentives.contract_addr, &[coin(funded_amt, "umars")]);

    // user deposits usdc
    red_bank.deposit(&mut mock_env, &user, coin(funded_amt, "uusdc")).unwrap();
    let balance = mock_env.query_balance(&user, "uusdc").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let mars_balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(mars_balance.amount, Uint128::zero());
    let user_collateral = red_bank.query_user_collateral(&mut mock_env, &user, "uusdc");
    assert_eq!(user_collateral.amount.u128(), funded_amt);

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::new(864000));

    //claim rewards
    incentives.claim_rewards(&mut mock_env, &user).unwrap();

    let balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(balance.amount, Uint128::new(864000));

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::zero());
}
