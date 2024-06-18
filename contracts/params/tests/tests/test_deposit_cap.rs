use std::str::FromStr;

use cosmwasm_std::{coins, Addr, Decimal, Uint128};
use mars_interest_rate::get_underlying_liquidity_amount;
use mars_params::{
    query::query_total_deposit,
    state::{ADDRESS_PROVIDER, ASSET_PARAMS},
};
use mars_testing::{mock_dependencies, mock_env_at_block_time};
use mars_types::{
    params::TotalDepositResponse,
    red_bank::{Market, UserDebtResponse},
};
use test_case::test_case;

use super::helpers::default_asset_params;

const CREDIT_MANAGER: &str = "credit_manager";
const MOCK_DENOM: &str = "utoken";
const LP_DENOM: &str =
    "factory/neutron1sf456kx85dz0wfjs4sx0s80dyzmc360pfc0rdzactxt8xrse9ykqsdpy2y/astroport/share";
const TIMESTAMP: u64 = 1690573960;

#[test_case(
    Market {
        denom: MOCK_DENOM.into(),
        collateral_total_scaled: Uint128::zero(),
        liquidity_index: Decimal::one(),
        indexes_last_updated: TIMESTAMP,
        ..Default::default()
    },
    UserDebtResponse {
        denom: MOCK_DENOM.into(),
        amount_scaled: Uint128::zero(),
        amount: Uint128::zero(),
        uncollateralized: true,
    },
    Uint128::zero(),
    Uint128::zero();
    "zero liquidity, zero debt, zero balance"
)]
#[test_case(
    Market {
        denom: MOCK_DENOM.into(),
        collateral_total_scaled: Uint128::new(6023580722925709342),
        liquidity_index: Decimal::from_str("1.010435027113017045").unwrap(),
        indexes_last_updated: 1690573862,
        ..Default::default()
    },
    UserDebtResponse {
        denom: MOCK_DENOM.into(),
        amount_scaled: Uint128::new(442125932248737808),
        amount: Uint128::new(459180188271),
        uncollateralized: true,
    },
    Uint128::new(1751191642),
    Uint128::zero();
    "real data queried from mainnet"
)]
#[test_case(
    Market {
        denom: LP_DENOM.into(),
        collateral_total_scaled: Uint128::new(6023580722925709342),
        liquidity_index: Decimal::from_str("1.010435027113017045").unwrap(),
        indexes_last_updated: 1690573862,
        ..Default::default()
    },
    UserDebtResponse {
        denom: LP_DENOM.into(),
        amount_scaled: Uint128::new(442125932248737808),
        amount: Uint128::new(459180188271),
        uncollateralized: true,
    },
    Uint128::new(1751191642),
    Uint128::new(1234560000);
    "real data queried from mainnet with astroport deposit"
)]
fn querying_total_deposit(
    rb_market: Market,
    rb_debt: UserDebtResponse,
    cm_balance: Uint128,
    astroport_deposit: Uint128,
) {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env_at_block_time(TIMESTAMP);

    let denom = rb_market.denom.clone();

    let params_unchecked = default_asset_params(&denom);
    let params = params_unchecked.check(deps.as_ref().api).unwrap();

    // setup
    deps.querier.set_redbank_market(rb_market.clone());
    deps.querier.set_red_bank_user_debt(CREDIT_MANAGER, rb_debt);
    deps.querier.update_balances(CREDIT_MANAGER, coins(cm_balance.u128(), &denom));
    deps.querier.set_astroport_deposit("incentives", &denom, astroport_deposit);
    ADDRESS_PROVIDER.save(deps.as_mut().storage, &Addr::unchecked("address_provider")).unwrap();
    ASSET_PARAMS.save(deps.as_mut().storage, &denom, &params).unwrap();

    // compute the correct, expected total deposit
    let rb_deposit =
        get_underlying_liquidity_amount(rb_market.collateral_total_scaled, &rb_market, TIMESTAMP)
            .unwrap();
    let exp_total_deposit = rb_deposit + cm_balance + astroport_deposit;

    // query total deposit
    let res = query_total_deposit(deps.as_ref(), &env, denom.clone()).unwrap();
    assert_eq!(
        res,
        TotalDepositResponse {
            denom,
            amount: exp_total_deposit,
            cap: params.deposit_cap,
        }
    );
}
