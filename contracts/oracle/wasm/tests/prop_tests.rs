use std::{cmp::min, collections::HashSet};

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

/// Generates a random Decimal between zero and `MAX_LIQ`.
pub fn decimal() -> impl Strategy<Value = Decimal> {
    (0..MAX_LIQ).prop_map(|x| Decimal::new(x.into()))
}

/// Generates a pair of u128s where the first is greater than the second. This is so we can swap
/// without exceeding max spread.
pub fn liquidity() -> impl Strategy<Value = [u128; 2]> {
    (1000000000..MAX_LIQ)
        .prop_flat_map(|v| (v..min(v * 10000, MAX_LIQ), Just(v / 10)))
        .prop_map(|(x, y)| [x, y])
}

/// Generates a vector of (denom, price) tuples that can be used as route assets for the price source.
pub fn route_prices(pair_denoms: [&str; 2]) -> impl Strategy<Value = Vec<(&'_ str, Decimal)>> {
    vec((denom(), decimal()), 0..4)
        .prop_flat_map(move |x| {
            let mut v = x;
            if !v.is_empty() {
                v[0].0 = pair_denoms[1];
            }
            Just(v)
        })
        .prop_filter("route assets must be unique", |v| {
            let mut set = HashSet::new();
            v.iter().all(|x| set.insert(x.0))
        })
        .prop_filter("route assets cannot contain the price source denom", move |v| {
            v.iter().all(|x| x.0 != pair_denoms[0])
        })
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
    (pair_denoms,route_prices) in pair_denoms().prop_flat_map(|pair_denoms|
      (Just(pair_denoms),route_prices(pair_denoms))
    ),
    initial_liq in liquidity(),
  ){
    let base_denom = if !route_prices.is_empty() {
      route_prices[route_prices.len() -1].0
    } else {
      pair_denoms[1]
    };
    validate_and_query_astroport_spot_price_source(pair_type, &pair_denoms, base_denom, &route_prices, &initial_liq, true);
  }

  #[test]
  fn proptest_validate_and_query_astroport_twap_price_source(
    pair_type in astro_pair_type(),
    (pair_denoms,route_prices) in pair_denoms().prop_flat_map(|pair_denoms|
      (Just(pair_denoms),route_prices(pair_denoms))
    ),
    initial_liq in liquidity(),
    (window_size,tolerance) in (2..1000000u64).prop_flat_map(|x| (Just(x), 0..x))
  ) {
    let base_denom = if !route_prices.is_empty() {
      route_prices[route_prices.len() -1].0
    } else {
      pair_denoms[1]
    };
    validate_and_query_astroport_twap_price_source(pair_type, &pair_denoms, base_denom, &route_prices, tolerance, window_size, &initial_liq);
  }
}
