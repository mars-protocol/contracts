use cosmwasm_std::{assert_approx_eq, coin, Addr, Decimal, Uint128};
use mars_testing::integration::mock_env::MockEnvBuilder;

use crate::helpers::default_asset_params;

mod helpers;

const ONE_WEEK_IN_SEC: u64 = 604800;

// Note: The incentives rewards for an individual is calculated as follows:
// (emissions_per_second) * (amount of seconds that the asset has been deposited into the redbank) * (amount of asset user deposited / total amount of asset deposited)
// this calculation is used to verify that the amount of rewards claimed is accurate in all the tests below

// User A deposits usdc in the redbank and claims rewards after one day
#[test]
fn rewards_claim() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();

    let (market_params, asset_params) = default_asset_params();
    red_bank.init_asset(&mut mock_env, "uusdc", market_params);
    params.init_params(&mut mock_env, "uusdc", asset_params);

    let incentives = mock_env.incentives.clone();
    incentives.whitelist_incentive_denoms(&mut mock_env, &[("umars", 3)]);
    incentives.init_asset_incentive_from_current_block(
        &mut mock_env,
        "uusdc",
        "umars",
        10,
        ONE_WEEK_IN_SEC,
    );

    let user = Addr::unchecked("user_a");
    let funded_amt = 10_000_000_000u128;
    mock_env.fund_account(&user, &[coin(funded_amt, "uusdc")]);

    red_bank.deposit(&mut mock_env, &user, coin(funded_amt, "uusdc")).unwrap();
    let balance = mock_env.query_balance(&user, "uusdc").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let mars_balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(mars_balance.amount, Uint128::zero());
    let user_collateral = red_bank.query_user_collateral(&mut mock_env, &user, "uusdc");
    assert_eq!(user_collateral.amount.u128(), funded_amt);

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance[0].amount, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance[0].amount, Uint128::new(864000));

    incentives.claim_rewards(&mut mock_env, &user).unwrap();

    let balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(balance.amount, Uint128::new(864000));
    let mars_balance = mock_env.query_balance(&incentives.contract_addr, "umars").unwrap();
    assert_eq!(mars_balance.amount, Uint128::from(ONE_WEEK_IN_SEC * 10 - 864000));

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance[0].amount, Uint128::zero());
}

// User A deposited usdc in the redbank when incentives were 5 emissions per second
// Then claimed rewards after one day
// Then user A later deposits osmo in the red bank when incentives were 10 emissions per second without withdrawing usdc
// Then claimed rewards after one day again
#[test]
fn emissions_rates() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();

    let (market_params, asset_params) = default_asset_params();
    red_bank.init_asset(&mut mock_env, "uusdc", market_params.clone());
    red_bank.init_asset(&mut mock_env, "uosmo", market_params.clone());
    red_bank.init_asset(&mut mock_env, "umars", market_params);
    params.init_params(&mut mock_env, "uusdc", asset_params.clone());
    params.init_params(&mut mock_env, "uosmo", asset_params.clone());
    params.init_params(&mut mock_env, "umars", asset_params);

    let incentives = mock_env.incentives.clone();
    incentives.whitelist_incentive_denoms(&mut mock_env, &[("umars", 3)]);
    incentives.init_asset_incentive_from_current_block(
        &mut mock_env,
        "uusdc",
        "umars",
        5,
        ONE_WEEK_IN_SEC,
    );

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
    assert_eq!(rewards_balance[0].amount, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance[0].amount, Uint128::new(432000)); // 86400*5

    incentives.claim_rewards(&mut mock_env, &user).unwrap();

    let balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(balance.amount, Uint128::new(432000));

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance[0].amount, Uint128::zero());

    incentives.init_asset_incentive_from_current_block(
        &mut mock_env,
        "uosmo",
        "umars",
        10,
        ONE_WEEK_IN_SEC,
    );

    red_bank.deposit(&mut mock_env, &user, coin(funded_amt, "uosmo")).unwrap();
    let balance = mock_env.query_balance(&user, "uosmo").unwrap();
    assert_eq!(balance.amount, Uint128::zero());
    let mars_balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(mars_balance.amount, Uint128::new(432000));
    let user_collateral = red_bank.query_user_collateral(&mut mock_env, &user, "uosmo");
    assert_eq!(user_collateral.amount.u128(), funded_amt);

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance[0].amount, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance[0].amount, Uint128::new(1296000)); // 432000 + (86400*10)

    incentives.claim_rewards(&mut mock_env, &user).unwrap();

    let balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(balance.amount, Uint128::new(1728000)); // 1296000 + 432000

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance[0].amount, Uint128::zero());
}

// User A deposits usdc in the redbank and claimed rewards after one day
// Then withdraws usdc and checks rewards balance after one day
#[test]
fn no_incentives_accrued_after_withdraw() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();

    let (market_params, asset_params) = default_asset_params();
    red_bank.init_asset(&mut mock_env, "uusdc", market_params.clone());
    red_bank.init_asset(&mut mock_env, "uosmo", market_params.clone());
    red_bank.init_asset(&mut mock_env, "umars", market_params);
    params.init_params(&mut mock_env, "uusdc", asset_params.clone());
    params.init_params(&mut mock_env, "uosmo", asset_params.clone());
    params.init_params(&mut mock_env, "umars", asset_params);

    let incentives = mock_env.incentives.clone();
    incentives.whitelist_incentive_denoms(&mut mock_env, &[("umars", 3)]);
    incentives.init_asset_incentive_from_current_block(
        &mut mock_env,
        "uusdc",
        "umars",
        5,
        ONE_WEEK_IN_SEC,
    );

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
    assert_eq!(rewards_balance[0].amount, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance[0].amount, Uint128::new(432000)); // 86400 * 5

    incentives.claim_rewards(&mut mock_env, &user).unwrap();

    let balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(balance.amount, Uint128::new(432000));

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance[0].amount, Uint128::zero());

    red_bank.withdraw(&mut mock_env, &user, "uusdc", None).unwrap();
    let balance = mock_env.query_balance(&user, "uusdc").unwrap();
    assert_eq!(balance.amount, Uint128::new(funded_amt));
    let mars_balance = mock_env.query_balance(&user, "umars").unwrap();
    assert_eq!(mars_balance.amount, Uint128::new(432000));
    let user_collateral = red_bank.query_user_collateral(&mut mock_env, &user, "uosmo");
    assert_eq!(user_collateral.amount, Uint128::zero());

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance[0].amount, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance[0].amount, Uint128::zero());
}

// User A deposits usdc, osmo, and atom all with different emissions per second & claims rewards after one day
#[test]
fn multiple_assets() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();

    let (market_params, asset_params) = default_asset_params();
    red_bank.init_asset(&mut mock_env, "uusdc", market_params.clone());
    red_bank.init_asset(&mut mock_env, "uosmo", market_params.clone());
    red_bank.init_asset(&mut mock_env, "uatom", market_params.clone());
    red_bank.init_asset(&mut mock_env, "umars", market_params);
    params.init_params(&mut mock_env, "uusdc", asset_params.clone());
    params.init_params(&mut mock_env, "uosmo", asset_params.clone());
    params.init_params(&mut mock_env, "uatom", asset_params.clone());
    params.init_params(&mut mock_env, "umars", asset_params);

    // set incentives
    let incentives = mock_env.incentives.clone();
    incentives.whitelist_incentive_denoms(&mut mock_env, &[("umars", 3)]);
    incentives.init_asset_incentive_from_current_block(
        &mut mock_env,
        "uusdc",
        "umars",
        5,
        ONE_WEEK_IN_SEC,
    );
    incentives.init_asset_incentive_from_current_block(
        &mut mock_env,
        "uatom",
        "umars",
        10,
        ONE_WEEK_IN_SEC,
    );
    incentives.init_asset_incentive_from_current_block(
        &mut mock_env,
        "uosmo",
        "umars",
        3,
        ONE_WEEK_IN_SEC,
    );

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
    assert_eq!(rewards_balance[0].amount, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user);
    assert_eq!(rewards_balance, Uint128::new(1555200));
}

// User A deposits half the amount user B deposits in the red bank
// User A withdraws usdc after one day while user B holds usdc in the red bank
#[test]
fn multiple_users() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();

    let (market_params, asset_params) = default_asset_params();
    red_bank.init_asset(&mut mock_env, "uusdc", market_params);
    params.init_params(&mut mock_env, "uusdc", asset_params);

    // set incentives
    let incentives = mock_env.incentives.clone();
    incentives.whitelist_incentive_denoms(&mut mock_env, &[("umars", 3)]);
    incentives.init_asset_incentive_from_current_block(
        &mut mock_env,
        "uusdc",
        "umars",
        5,
        ONE_WEEK_IN_SEC,
    );

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
    assert_eq!(rewards_balance[0].amount, Uint128::zero());

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_b);
    assert_eq!(rewards_balance[0].amount, Uint128::zero());

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_a);
    assert_eq!(rewards_balance[0].amount, Uint128::new(144000)); // (86400*5) * (1/3)

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_b);
    assert_eq!(rewards_balance[0].amount, Uint128::new(288000)); // (86400*5)/2 * (2/3)

    // User A withdraws, user B holds

    red_bank.withdraw(&mut mock_env, &user_a, "uusdc", None).unwrap();

    mock_env.increment_by_time(86400); // 24 hours

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_a);
    assert_eq!(rewards_balance[0].amount, Uint128::new(144000)); // stays the same

    let rewards_balance = incentives.query_unclaimed_rewards(&mut mock_env, &user_b);
    assert_eq!(rewards_balance[0].amount, Uint128::new(720000)); // 288000 + (86400*5)
}

// Rewards are proportionally distributed among users.
// rewards-collector contract accrues rewards.
// All mars is used from incentives contract.
#[test]
fn rewards_distributed_among_users_and_rewards_collector() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    // setup oracle prices
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_fixed(&mut mock_env, "uusdc", Decimal::from_ratio(15u128, 10u128));
    oracle.set_price_source_fixed(&mut mock_env, "uatom", Decimal::from_ratio(10u128, 1u128));

    // setup red-bank assets
    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();

    let (market_params, asset_params) = default_asset_params();
    red_bank.init_asset(&mut mock_env, "uusdc", market_params.clone());
    red_bank.init_asset(&mut mock_env, "uosmo", market_params.clone());
    red_bank.init_asset(&mut mock_env, "uatom", market_params);
    params.init_params(&mut mock_env, "uusdc", asset_params.clone());
    params.init_params(&mut mock_env, "uosmo", asset_params.clone());
    params.init_params(&mut mock_env, "uatom", asset_params);

    // fund user accounts
    let user_a = Addr::unchecked("user_a");
    mock_env.fund_account(&user_a, &[coin(1_000_000_000_000u128, "uusdc")]);
    let user_b = Addr::unchecked("user_b");
    mock_env.fund_account(&user_b, &[coin(1_000_000_000_000u128, "uusdc")]);
    mock_env.fund_account(&user_b, &[coin(1_000_000_000_000u128, "uatom")]);

    // users deposit assets
    let user_a_uusdc_deposited_amt = 150_000_000_000u128;
    red_bank.deposit(&mut mock_env, &user_a, coin(user_a_uusdc_deposited_amt, "uusdc")).unwrap();
    let user_b_uusdc_deposited_amt = 300_000_000_000u128;
    red_bank.deposit(&mut mock_env, &user_b, coin(user_b_uusdc_deposited_amt, "uusdc")).unwrap();
    let user_b_uatom_deposited_amt = 6_000_000_000u128;
    red_bank.deposit(&mut mock_env, &user_b, coin(user_b_uatom_deposited_amt, "uatom")).unwrap();

    // set incentives
    let umars_eps_for_uusdc = 150000;
    let umars_eps_for_uosmo = 730000;
    let umars_eps_for_uatom = 310000;
    let incentive_duration_sec = 2592000u64;
    let incentives = mock_env.incentives.clone();
    incentives.whitelist_incentive_denoms(&mut mock_env, &[("umars", 3)]);
    incentives.init_asset_incentive_from_current_block(
        &mut mock_env,
        "uusdc",
        "umars",
        umars_eps_for_uusdc,
        incentive_duration_sec,
    );
    incentives.init_asset_incentive_from_current_block(
        &mut mock_env,
        "uosmo",
        "umars",
        umars_eps_for_uosmo,
        incentive_duration_sec,
    );
    incentives.init_asset_incentive_from_current_block(
        &mut mock_env,
        "uatom",
        "umars",
        umars_eps_for_uatom,
        incentive_duration_sec,
    );

    // calculate how much umars is need for incentives for uusdc and uatom (only these assets are deposited in red-bank)
    let umars_incentives_amt = (umars_eps_for_uusdc + umars_eps_for_uosmo + umars_eps_for_uatom)
        * (incentive_duration_sec as u128);

    // fund incentives contract
    // mock_env.fund_account(&incentives.contract_addr, &[coin(umars_incentives_amt, "umars")]);
    let balance = mock_env.query_balance(&incentives.contract_addr, "umars").unwrap();
    assert_eq!(balance.amount, Uint128::new(umars_incentives_amt));

    // move few blocks
    mock_env.increment_by_time(60);

    // user_a borrows uusdc and uatom
    red_bank.borrow(&mut mock_env, &user_a, "uusdc", 10_000_000_000u128).unwrap();
    red_bank.borrow(&mut mock_env, &user_a, "uatom", 1_000_000_000u128).unwrap();

    // move few blocks
    mock_env.increment_by_time(400000);

    // user_a borrows more uusdc and uatom
    red_bank.borrow(&mut mock_env, &user_a, "uusdc", 100_000_000u128).unwrap();
    red_bank.borrow(&mut mock_env, &user_a, "uatom", 100_000_000u128).unwrap();

    // let's finish current incentives
    mock_env.increment_by_time(incentive_duration_sec);

    // uusdc and uatom rewards should be accrued for rewards-collector
    let rewards_collector = mock_env.rewards_collector.clone();
    let uusdc_collateral_rc =
        red_bank.query_user_collateral(&mut mock_env, &rewards_collector.contract_addr, "uusdc");
    assert_eq!(uusdc_collateral_rc.amount, Uint128::new(9_024_438u128));
    let uatom_collateral_rc =
        red_bank.query_user_collateral(&mut mock_env, &rewards_collector.contract_addr, "uatom");
    assert_eq!(uatom_collateral_rc.amount, Uint128::new(1_461_774u128));
    let uosmo_collateral_rc =
        red_bank.query_user_collateral(&mut mock_env, &rewards_collector.contract_addr, "uosmo");
    assert_eq!(uosmo_collateral_rc.amount, Uint128::zero());

    // rewards-collector accrue rewards
    let rewards_balance_rc =
        incentives.query_unclaimed_rewards(&mut mock_env, &rewards_collector.contract_addr);
    assert!(!rewards_balance_rc.is_empty());
    println!("rewards_balance_rc: {:?}", rewards_balance_rc);

    // sum of unclaimed rewards should be equal to total umars available for finished incentive
    let rewards_balance_user_a = incentives.query_unclaimed_rewards(&mut mock_env, &user_a);
    println!("rewards_balance_user_a: {:?}", rewards_balance_user_a);
    let rewards_balance_user_b = incentives.query_unclaimed_rewards(&mut mock_env, &user_b);
    println!("rewards_balance_user_b: {:?}", rewards_balance_user_b);
    let total_claimed_rewards = rewards_balance_rc[0].amount
        + rewards_balance_user_a[0].amount
        + rewards_balance_user_b[0].amount;
    // ~ values very close (small difference due to rounding errors for index calculation)
    assert_approx_eq!(
        total_claimed_rewards.u128(),
        umars_incentives_amt - umars_eps_for_uosmo * incentive_duration_sec as u128,
        "0.00001"
    );

    // users claim rewards
    incentives.claim_rewards(&mut mock_env, &user_a).unwrap();
    let umars_balance_user_a = mock_env.query_balance(&user_a, "umars").unwrap();
    assert_eq!(vec![umars_balance_user_a], rewards_balance_user_a);
    incentives.claim_rewards(&mut mock_env, &user_b).unwrap();
    let umars_balance_user_b = mock_env.query_balance(&user_b, "umars").unwrap();
    assert_eq!(vec![umars_balance_user_b], rewards_balance_user_b);

    // rewards-collector claims rewards
    rewards_collector.claim_incentive_rewards(&mut mock_env).unwrap();
    let umars_balance_rc =
        mock_env.query_balance(&rewards_collector.contract_addr, "umars").unwrap();
    assert_eq!(vec![umars_balance_rc], rewards_balance_rc);
}
