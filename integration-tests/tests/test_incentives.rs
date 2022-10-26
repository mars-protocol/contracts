use cosmwasm_std::{Addr, BlockInfo, coin, Uint128, Uint64};

use mars_testing::integration::mock_env::MockEnvBuilder;

use crate::helpers::{default_asset_params};

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
    let funded_usdc = 10_000_000_000u128;
    mock_env.fund_account(&user, &[coin(funded_usdc, "uusdc")]);

    // user deposits usdc
    red_bank.deposit(&mut mock_env, &user, coin(funded_usdc, "uusdc")).unwrap();
    let balance = mock_env.query_balance(&user, "uusdc").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let user_collateral = red_bank.query_user_collateral(&mut mock_env, &user, "uusdc");
    assert_eq!(user_collateral.amount.u128(), funded_usdc);

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::zero());

    let info = mock_env.app.block_info();
    mock_env.app.set_block(BlockInfo{
        height: info.height + 100,
        time: info.time.plus_seconds(86400), // 24 hours
        chain_id: info.chain_id,
    });

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::new(86400 * 10));

    //claim rewards
    incentives.claim_rewards(&mut mock_env, &user).unwrap();

    let balance = mock_env.query_balance(&user, "uusdc").unwrap();
    assert_eq!(balance.amount, Uint128::new(86400 * 10));

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::zero());
}
