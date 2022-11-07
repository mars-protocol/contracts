use cosmwasm_std::{coin, Addr, Uint128};

use mars_testing::integration::mock_env::MockEnvBuilder;

use crate::helpers::default_asset_params;

mod helpers;

//Note: The incentives rewards for an indivdiual is calculated as follows:
// (emissions_per_second) * (amount of seconds that the asset has been deposited into the redbank) * (amount of asset user deposited/ total amount of asset deposited)
// this calculation is used to verify that the amount of rewards claimed is accurate in all the tests below

#[test]
//User A deposits usdc in the redbank and claims rewards after one day
fn test_rewards_claim() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    red_bank.init_asset(&mut mock_env, "uusdc", default_asset_params());

    let incentives = mock_env.incentives.clone();
    incentives.set_asset_incentive(&mut mock_env, "uusdc", 10);

    let user = Addr::unchecked("user_a");
    let funded_amt = 10_000_000_000u128;
    mock_env.fund_account(&user, &[coin(funded_amt, "uusdc")]);

    mock_env.fund_account(&incentives.contract_addr, &[coin(funded_amt, "umars")]);

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

    incentives.claim_rewards(&mut mock_env, &user).unwrap();

    let balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(balance.amount, Uint128::new(864000));

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::zero());
}

#[test]
// User A deposited usdc in the redbank when incentives were 5 emissions per second
// Then claimed rewards after one day
// Then user A later deposits osmo in the red bank when incentives were 10 emissions per second without withdrawing usdc
// Then claimed rewards after one day again
fn test_emissions_rates() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    red_bank.init_asset(&mut mock_env, "uusdc", default_asset_params());
    red_bank.init_asset(&mut mock_env, "uosmo", default_asset_params());
    red_bank.init_asset(&mut mock_env, "umars", default_asset_params());

    let incentives = mock_env.incentives.clone();
    incentives.set_asset_incentive(&mut mock_env, "uusdc", 5);

    let user = Addr::unchecked("user_a");
    let funded_amt = 10_000_000_000u128;
    mock_env.fund_account(&user, &[coin(funded_amt, "uusdc")]);
    mock_env.fund_account(&user, &[coin(funded_amt, "uosmo")]);

    mock_env.fund_account(&incentives.contract_addr, &[coin(funded_amt, "umars")]);

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
    assert_eq!(rewards_balance, Uint128::new(432000)); // 86400*5

    incentives.claim_rewards(&mut mock_env, &user).unwrap();

    let balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(balance.amount, Uint128::new(432000));

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::zero());

    incentives.set_asset_incentive(&mut mock_env, "uosmo", 10);

    red_bank.deposit(&mut mock_env, &user, coin(funded_amt, "uosmo")).unwrap();
    let balance = mock_env.query_balance(&user, "uosmo").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let mars_balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(mars_balance.amount, Uint128::new(432000));
    let user_collateral = red_bank.query_user_collateral(&mut mock_env, &user, "uosmo");
    assert_eq!(user_collateral.amount.u128(), funded_amt);

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::new(1296000)); // 432000 + (86400*10)

    incentives.claim_rewards(&mut mock_env, &user).unwrap();

    let balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(balance.amount, Uint128::new(1728000)); // 1296000 + 432000

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::zero());
}

#[test]
// User A deposits usdc in the redbank and claimed rewards after one day
// Then withdraws usdc and checks rewards balance after one day
fn test_withdraw() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    red_bank.init_asset(&mut mock_env, "uusdc", default_asset_params());
    red_bank.init_asset(&mut mock_env, "uosmo", default_asset_params());
    red_bank.init_asset(&mut mock_env, "umars", default_asset_params());

    let incentives = mock_env.incentives.clone();
    incentives.set_asset_incentive(&mut mock_env, "uusdc", 5);

    let user = Addr::unchecked("user_a");
    let funded_amt = 10_000_000_000u128;
    mock_env.fund_account(&user, &[coin(funded_amt, "uusdc")]);
    mock_env.fund_account(&user, &[coin(funded_amt, "uosmo")]);

    mock_env.fund_account(&incentives.contract_addr, &[coin(funded_amt, "umars")]);

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
    assert_eq!(rewards_balance, Uint128::new(432000)); // 86400 * 5

    incentives.claim_rewards(&mut mock_env, &user).unwrap();

    let balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(balance.amount, Uint128::new(432000));

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::zero());

    red_bank.withdraw(&mut mock_env, &user, "uusdc", None).unwrap();
    let balance = mock_env.query_balance(&user, "uusdc").unwrap();
    assert_eq!(balance.amount, Uint128::new(funded_amt));
    let mars_balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(mars_balance.amount, Uint128::new(432000));
    let user_collateral = red_bank.query_user_collateral(&mut mock_env, &user, "uosmo");
    assert_eq!(user_collateral.amount, Uint128::zero());

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::zero());
}

#[test]
// User A deposits usdc, osmo, and atom all with different emissions per second & claims rewards after one day
fn test_multiple_assets() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    red_bank.init_asset(&mut mock_env, "uusdc", default_asset_params());
    red_bank.init_asset(&mut mock_env, "uosmo", default_asset_params());
    red_bank.init_asset(&mut mock_env, "uatom", default_asset_params());
    red_bank.init_asset(&mut mock_env, "umars", default_asset_params());

    // set incentives
    let incentives = mock_env.incentives.clone();
    incentives.set_asset_incentive(&mut mock_env, "uusdc", 5);
    incentives.set_asset_incentive(&mut mock_env, "uatom", 10);
    incentives.set_asset_incentive(&mut mock_env, "uosmo", 3);

    // fund user wallet account
    let user = Addr::unchecked("user_a");
    let funded_amt = 10_000_000_000u128;
    mock_env.fund_account(&user, &[coin(funded_amt, "uusdc")]);
    mock_env.fund_account(&user, &[coin(funded_amt, "uosmo")]);
    mock_env.fund_account(&user, &[coin(funded_amt, "uatom")]);

    // fund incentives contract
    mock_env.fund_account(&incentives.contract_addr, &[coin(funded_amt, "umars")]);

    // user deposits assets
    red_bank.deposit(&mut mock_env, &user, coin(funded_amt, "uusdc")).unwrap();
    red_bank.deposit(&mut mock_env, &user, coin(funded_amt, "uatom")).unwrap();
    red_bank.deposit(&mut mock_env, &user, coin(funded_amt, "uosmo")).unwrap();
    let balance = mock_env.query_balance(&user, "uusdc").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let balance = mock_env.query_balance(&user, "uatom").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let balance = mock_env.query_balance(&user, "uosmo").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let mars_balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(mars_balance.amount, Uint128::zero());
    let user_collateral = red_bank.query_user_collateral(&mut mock_env, &user, "uusdc");
    assert_eq!(user_collateral.amount.u128(), funded_amt);
    let user_collateral = red_bank.query_user_collateral(&mut mock_env, &user, "uatom");
    assert_eq!(user_collateral.amount.u128(), funded_amt);
    let user_collateral = red_bank.query_user_collateral(&mut mock_env, &user, "uosmo");
    assert_eq!(user_collateral.amount.u128(), funded_amt);

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::new(1555200));
}

#[test]
// User A holds usdc in the red bank while there are incentives then incentives are stopped and then incentives are restarted

fn test_stopping_incentives() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    red_bank.init_asset(&mut mock_env, "uusdc", default_asset_params());

    // set incentives
    let incentives = mock_env.incentives.clone();
    incentives.set_asset_incentive(&mut mock_env, "uusdc", 5);

    // fund user wallet account
    let user = Addr::unchecked("user_a");
    let funded_amt = 10_000_000_000u128;
    mock_env.fund_account(&user, &[coin(funded_amt, "uusdc")]);

    // fund incentives contract
    mock_env.fund_account(&incentives.contract_addr, &[coin(funded_amt, "umars")]);

    // user deposits assets
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
    assert_eq!(rewards_balance, Uint128::new(432000));

    //stop incentives
    incentives.set_asset_incentive(&mut mock_env, "uusdc", 0);

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::new(432000));

    // restart incentives
    incentives.set_asset_incentive(&mut mock_env, "uusdc", 5);

    mock_env.increment_by_time(43200); // 12 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::new(648000)); // (5*86400) + (5*43200)
}

#[test]
// User A deposits half the amount user B deposits in the red bank
// User A withdraws usdc after one day while user B holds usdc in the red bank
fn test_multiple_users() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    red_bank.init_asset(&mut mock_env, "uusdc", default_asset_params());

    // set incentives
    let incentives = mock_env.incentives.clone();
    incentives.set_asset_incentive(&mut mock_env, "uusdc", 5);

    // fund user wallet account
    let user_a = Addr::unchecked("user_a");
    let user_b = Addr::unchecked("user_b");
    let funded_amt_one = 10_000_000_000u128;
    let funded_amt_two = 20_000_000_000u128;
    mock_env.fund_account(&user_a, &[coin(funded_amt_one, "uusdc")]);
    mock_env.fund_account(&user_b, &[coin(funded_amt_two, "uusdc")]);

    // fund incentives contract
    mock_env.fund_account(&incentives.contract_addr, &[coin(funded_amt_two, "umars")]);

    // user deposits assets
    red_bank.deposit(&mut mock_env, &user_a, coin(funded_amt_one, "uusdc")).unwrap();
    red_bank.deposit(&mut mock_env, &user_b, coin(funded_amt_two, "uusdc")).unwrap();
    let balance = mock_env.query_balance(&user_a, "uusdc").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let mars_balance = mock_env.query_balance(&user_a, "umars").unwrap();
    assert_eq!(mars_balance.amount, Uint128::zero());
    let user_collateral = red_bank.query_user_collateral(&mut mock_env, &user_a, "uusdc");
    assert_eq!(user_collateral.amount.u128(), funded_amt_one);

    let balance = mock_env.query_balance(&user_b, "uusdc").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let mars_balance = mock_env.query_balance(&user_b, "umars").unwrap();
    assert_eq!(mars_balance.amount, Uint128::zero());
    let user_collateral = red_bank.query_user_collateral(&mut mock_env, &user_b, "uusdc");
    assert_eq!(user_collateral.amount.u128(), funded_amt_two);

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_a);
    assert_eq!(rewards_balance, Uint128::zero());

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_b);
    assert_eq!(rewards_balance, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_a);
    assert_eq!(rewards_balance, Uint128::new(144000)); // (86400*5) * (1/3)

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_b);
    assert_eq!(rewards_balance, Uint128::new(288000)); // (86400*5)/2 * (2/3)

    // User A withdraws, user B holds

    red_bank.withdraw(&mut mock_env, &user_a, "uusdc", None).unwrap();

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_a);
    assert_eq!(rewards_balance, Uint128::new(144000)); // stays the same

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_b);
    assert_eq!(rewards_balance, Uint128::new(720000)); // 288000 + (86400*5)
}

#[test]
// User A attempts to claim rewards but there is not enough mars in the incentives contract
fn test_insufficient_mars() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    red_bank.init_asset(&mut mock_env, "uusdc", default_asset_params());

    //set incentives
    let incentives = mock_env.incentives.clone();
    incentives.set_asset_incentive(&mut mock_env, "uusdc", 5);

    // fund user wallet accounti
    let user_a = Addr::unchecked("user_a");
    let funded_amt_one = 10_000_000_000u128;
    let funded_amt_two = 500_000u128;
    mock_env.fund_account(&user_a, &[coin(funded_amt_one, "uusdc")]);

    // fund incentives contract
    mock_env.fund_account(&incentives.contract_addr, &[coin(funded_amt_two, "umars")]);

    // user deposits assets
    red_bank.deposit(&mut mock_env, &user_a, coin(funded_amt_one, "uusdc")).unwrap();
    let balance = mock_env.query_balance(&user_a, "uusdc").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let mars_balance = mock_env.query_balance(&user_a, "umars").unwrap();
    assert_eq!(mars_balance.amount, Uint128::zero());
    let user_collateral = red_bank.query_user_collateral(&mut mock_env, &user_a, "uusdc");
    assert_eq!(user_collateral.amount.u128(), funded_amt_one);

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_a);
    assert_eq!(rewards_balance, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_a);
    assert_eq!(rewards_balance, Uint128::new(432000)); // (86400*5)

    incentives.claim_rewards(&mut mock_env, &user_a).unwrap();

    let balance = mock_env.query_balance(&user_a, "umars").unwrap();
    assert_eq!(balance.amount, Uint128::new(432000));

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_a);
    assert_eq!(rewards_balance, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_a);
    assert_eq!(rewards_balance, Uint128::new(432000)); // (86400*5)

    incentives.claim_rewards(&mut mock_env, &user_a).unwrap_err();

    let balance = mock_env.query_balance(&user_a, "umars").unwrap();
    assert_eq!(balance.amount, Uint128::new(432000)); // balance previously claimed

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_a);
    assert_eq!(rewards_balance, Uint128::new(432000)); // newly accrued rewards unable to claim
}
