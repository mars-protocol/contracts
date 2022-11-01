use crate::helpers::default_asset_params;
use cosmwasm_std::{Addr, Decimal, Uint128};
use mars_testing::integration::mock_env::MockEnvBuilder;
use osmosis_std::types::osmosis::gamm::v1beta1::QuerySpotPriceResponse;
use osmosis_std::types::osmosis::twap::v1beta1::ArithmeticTwapToNowResponse;

mod helpers;

#[test]
fn spot_test() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    let red_bank = mock_env.red_bank.clone();
    red_bank.init_asset(&mut mock_env, "uatom", default_asset_params());
    //set up oracle with SPOT price source
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_spot(&mut mock_env, "uatom", 1);

    println!("helllo");
    // let source = oracle.query_price_source(&mut mock_env, "uatom");
    // println!("{}", source);
    //
    // //set spot price in env
    // mock_env.set_spot_price(
    //     1,
    //     "uatom",
    //     "uosmo",
    //     QuerySpotPriceResponse {
    //         spot_price: Decimal::from_ratio(12345u128, 77777u128).to_string(),
    //     },
    // );
    //
    // //check asset price
    // let atom_price = oracle.query_asset_price(&mut mock_env, "uatom");
    // assert_eq!(atom_price, Uint128::zero());
}

fn twap_test() {
    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();

    //setup oracle with TWAP price source
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_twap(&mut mock_env, "uatom", 1, 1800);
}
