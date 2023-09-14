use std::cmp::min;

use astroport::factory::PairType;
use cosmwasm_std::Decimal;
use mars_testing::wasm_oracle::{
    validate_and_query_astroport_spot_price_source, validate_and_query_astroport_twap_price_source,
};
use proptest::{collection::vec, prelude::*, proptest};

const MAX_LIQ: u128 = 1000000000000000000000000u128;

/// Generates a random Astroport PairType
pub fn astro_pair_type() -> impl Strategy<Value = PairType> {
    prop_oneof![Just(PairType::Xyk {}), Just(PairType::Stable {})]
}

/// Generates a random native denom
pub fn denom() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("uatom"), Just("uosmo"), Just("uion"), Just("stake")]
}

/// Generates a pair of unique denoms
pub fn pair_denoms() -> impl Strategy<Value = [&'static str; 2]> {
    vec(denom(), 2)
        .prop_map(|v| [v[0], v[1]])
        .prop_filter("pair denoms must be unique", |v| v[0] != v[1])
}

/// Generates a random Decimal between Decimal::one() and `MAX_LIQ`.
pub fn decimal() -> impl Strategy<Value = Decimal> {
    (Decimal::one().atomics().u128()..MAX_LIQ).prop_map(|x| Decimal::new(x.into()))
}

/// Generates a pair of u128s where the first is greater than the second. This is so we can swap
/// without exceeding max spread.
pub fn liquidity() -> impl Strategy<Value = [u128; 2]> {
    (1000000000..MAX_LIQ)
        .prop_flat_map(|v| (v..min(v * 10000, MAX_LIQ), Just(v / 10)))
        .prop_map(|(x, y)| [x, y])
}

proptest! {
  #![proptest_config(ProptestConfig {
      cases: 256,
      max_local_rejects: 100000,
      max_global_rejects: 100000,
      max_shrink_iters: 512,
      ..ProptestConfig::default()
  })]

  #[test]
  fn proptest_validate_and_query_astroport_spot_price_source(
    pair_type in astro_pair_type(),
    pair_denoms in pair_denoms(),
    base_denom in denom(),
    other_asset_price in decimal(),
    initial_liq in liquidity(),
  ){
    let register_second_price = !pair_denoms.contains(&base_denom);
    let other_asset_price = if register_second_price {
      Some(other_asset_price)
    } else {
      None
    };

    validate_and_query_astroport_spot_price_source(pair_type, &pair_denoms, base_denom, other_asset_price, &initial_liq, register_second_price, &[6,6]);
  }

  #[test]
  fn proptest_validate_and_query_astroport_twap_price_source(
    pair_type in astro_pair_type(),
    pair_denoms in pair_denoms(),
    base_denom in denom(),
    other_asset_price in decimal(),
    initial_liq in liquidity(),
    (window_size,tolerance) in (2..1000000u64).prop_flat_map(|x| (Just(x), 0..x))
  ) {
    let register_second_price = !pair_denoms.contains(&base_denom);
    let other_asset_price = if register_second_price {
      Some(other_asset_price)
    } else {
      None
    };
    validate_and_query_astroport_twap_price_source(pair_type, &pair_denoms, base_denom, other_asset_price, register_second_price, tolerance, window_size, &initial_liq, &[6,6]);
  }
}
