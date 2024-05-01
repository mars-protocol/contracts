use cosmwasm_std::{coin, Addr, Uint128};
use mars_testing::integration::{
    helpers::{osmo_asset_params, usdc_asset_params},
    mock_env::MockEnvBuilder,
};
use mars_types::params::TotalDepositResponse;

#[test]
fn should_return_total_deposits() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();

    let funded_amount = Uint128::new(10_000_000_000);
    let rb_osmo_amount = Uint128::new(2_345_678_900);
    let rb_usdc_amount = Uint128::new(1_234_567_890);

    let provider = Addr::unchecked("provider");
    let credit_manager = mock_env.credit_manager.clone();

    let osmo_denom = "uosmo";
    let uusdc_denom = "uusdc";

    let (market_params, asset_params) = osmo_asset_params();
    red_bank.init_asset(&mut mock_env, &asset_params.denom, market_params);
    params.init_params(&mut mock_env, asset_params);
    let (market_params, asset_params) = usdc_asset_params();
    red_bank.init_asset(&mut mock_env, &asset_params.denom, market_params);
    params.init_params(&mut mock_env, asset_params);

    mock_env.fund_accounts(&[&provider], funded_amount.u128(), &[osmo_denom, uusdc_denom]);

    mock_env.fund_accounts(&[&credit_manager], funded_amount.u128(), &[uusdc_denom]);

    red_bank.deposit(&mut mock_env, &provider, coin(rb_osmo_amount.u128(), osmo_denom)).unwrap();
    red_bank.deposit(&mut mock_env, &provider, coin(rb_usdc_amount.u128(), uusdc_denom)).unwrap();

    let res = params.all_total_deposits_v2(&mut mock_env, None, None);

    assert_eq!(
        res.data,
        vec![
            TotalDepositResponse {
                denom: osmo_denom.to_string(),
                amount: rb_osmo_amount,
                cap: Uint128::MAX
            },
            TotalDepositResponse {
                denom: uusdc_denom.to_string(),
                amount: funded_amount.checked_add(rb_usdc_amount).unwrap(),
                cap: Uint128::MAX
            }
        ]
    );

    assert!(!res.metadata.has_more);
}
